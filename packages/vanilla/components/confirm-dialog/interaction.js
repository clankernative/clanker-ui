const ROOT = '[data-cui-component="confirm-dialog"]';
const DIALOG = '[data-cui-confirm-dialog]';
const installations = new WeakMap();
const handledClicks = new WeakSet();
const FAILURE_LIMIT = 200;
const connected = (node, root) => !!node && (typeof node.isConnected === 'boolean' ? node.isConnected : !!root.contains?.(node));
const member = (root, node) => !!node && (root === node || !!root.contains?.(node));
function parts(node) {
  const component = node?.closest?.(ROOT);
  if (!component) return null;
  const trigger = component.querySelector('[data-cui-confirm-trigger]');
  const dialog = component.querySelector(DIALOG);
  const form = trigger?.form;
  if (!trigger || !dialog || !form || trigger.getAttribute('form') !== form.id) return null;
  return { component, trigger, dialog, form };
}
function validOptions(options) {
  if (options === undefined) return { submitPort: null, failureLabel: 'Unable to complete this action. Please try again.' };
  if (!options || typeof options !== 'object' || Array.isArray(options) || Object.keys(options).some(k => !['submit', 'failureLabel'].includes(k))) throw new TypeError('confirm-dialog install options accept only submit(request) and failureLabel');
  if (options.submit !== undefined && typeof options.submit !== 'function') throw new TypeError('confirm-dialog submit port must be a function');
  const label = options.failureLabel === undefined ? 'Unable to complete this action. Please try again.' : options.failureLabel;
  if (typeof label !== 'string' || !label.trim() || label.trim().length > FAILURE_LIMIT) throw new TypeError(`confirm-dialog failureLabel must be nonblank plaintext of at most ${FAILURE_LIMIT} characters`);
  return { submitPort: options.submit || null, failureLabel: label.trim() };
}
function clearFeedback(state) {
  if (state.feedback) { state.feedback.remove?.(); state.feedback = null; }
}
function showFeedback(state, label, dialog) {
  let feedback = state.feedback;
  if (!feedback || !dialog.contains?.(feedback)) {
    feedback = dialog.ownerDocument?.createElement?.('p');
    if (!feedback) return;
    feedback.setAttribute('data-cui-confirm-feedback', '');
    feedback.setAttribute('role', 'alert');
    feedback.setAttribute('aria-live', 'polite');
    dialog.append(feedback);
    state.feedback = feedback;
  }
  feedback.textContent = label;
}
function disposeState(dialog, state, restoreFocus) {
  state.controller.abort();
  clearFeedback(state);
  if (restoreFocus && connected(state.trigger, state.root)) state.trigger.focus?.();
  state.owned.delete(dialog);
  state.states.delete(dialog);
}
/**
 * Install confirmation behavior under root. The native submit button is the honest no-enhancement
 * fallback; the optional app-owned submit port receives confirmation intent, never authorization.
 * @param {Document|Element} root
 * @param {{submit?: (request: {form: HTMLFormElement, submitter: HTMLButtonElement, signal: AbortSignal}) => void|Promise<void>, failureLabel?: string}} [options]
 * @returns {() => void} Idempotent disposer. Teardown aborts pending app work and does not restore focus.
 */
export function install(root = document, options) {
  const { submitPort, failureLabel } = validOptions(options);
  const previous = installations.get(root);
  if (previous) {
    if (previous.submitPort !== submitPort || previous.failureLabel !== failureLabel) throw new Error('confirm-dialog root already has different options installed');
    return previous.dispose;
  }
  const states = new WeakMap(), owned = new Set();
  const alive = (dialog, state) => states.get(dialog) === state && !state.controller.signal.aborted && connected(dialog, root) && member(root, state.component) && parts(dialog)?.dialog === dialog;
  const cleanupDetached = () => {
    for (const dialog of [...owned]) {
      const state = states.get(dialog);
      if (state && (!connected(dialog, root) || !member(root, state.component) || parts(dialog)?.dialog !== dialog)) disposeState(dialog, state, false);
    }
  };
  const onClick = (event) => {
    if (handledClicks.has(event)) return;
    const trigger = event.target?.closest?.('[data-cui-confirm-trigger]');
    if (trigger) {
      const p = parts(trigger);
      if (!p || !member(root, p.component) || !connected(p.dialog, root) || typeof p.dialog.showModal !== 'function' || typeof AbortController !== 'function' || (!submitPort && typeof p.form.requestSubmit !== 'function') || p.dialog.open || states.has(p.dialog)) return;
      if (typeof p.form.checkValidity === 'function' && !p.form.checkValidity()) {
        p.form.reportValidity?.();
        return;
      }
      try {
        p.dialog.showModal();
        if (!p.dialog.open) return;
        const state = { root, states, owned, component: p.component, trigger, form: p.form, controller: new AbortController(), pending: false, feedback: null, validationClose: false };
        states.set(p.dialog, state); owned.add(p.dialog);
        handledClicks.add(event);
        event.preventDefault();
      } catch (_) { /* preserve ordinary form submission */ }
      return;
    }
    const cancel = event.target?.closest?.('[data-cui-confirm-cancel]');
    if (cancel) { const p = parts(cancel); if (p && member(root, p.component) && p.dialog.open && typeof p.dialog.close === 'function') { handledClicks.add(event); p.dialog.close(); } return; }
    const accept = event.target?.closest?.('[data-cui-confirm-accept]');
    if (accept) {
      const p = parts(accept), state = p && states.get(p.dialog);
      if (!p || !member(root, p.component) || !state || state.form !== p.form || !p.dialog.open || state.pending || state.controller.signal.aborted) return;
      clearFeedback(state);
      if (typeof p.form.checkValidity === 'function' && !p.form.checkValidity()) {
        state.validationClose = true;
        state.controller.abort();
        if (typeof p.dialog.close === 'function') p.dialog.close();
        if (state.validationClose) { disposeState(p.dialog, state, false); state.validationClose = false; }
        p.form.reportValidity?.();
        return;
      }
      state.pending = true;
      const request = { form: state.form, submitter: state.trigger, signal: state.controller.signal };
      let result;
      try { result = submitPort ? submitPort(request) : state.form.requestSubmit(state.trigger); }
      catch (_) { if (alive(p.dialog, state)) { state.pending = false; showFeedback(state, failureLabel, p.dialog); } return; }
      Promise.resolve(result).then(() => {
        if (!alive(p.dialog, state) || !p.dialog.open) return;
        if (typeof p.dialog.close === 'function') p.dialog.close();
      }, () => { if (alive(p.dialog, state)) { state.pending = false; showFeedback(state, failureLabel, p.dialog); } });
      return;
    }
    const dialog = event.target?.closest?.(DIALOG);
    if (dialog && event.target === dialog && dialog.open && typeof dialog.close === 'function' && member(root, dialog)) dialog.close();
  };
  const onClose = (event) => {
    const dialog = event.target;
    if (!dialog?.matches?.(DIALOG)) return;
    const state = states.get(dialog); if (!state) return;
    const validationClose = state.validationClose;
    disposeState(dialog, state, !validationClose);
  };
  root.addEventListener('click', onClick); root.addEventListener('close', onClose, true);
  const observer = typeof MutationObserver === 'function' ? new MutationObserver(cleanupDetached) : null;
  observer?.observe(root, { childList: true, subtree: true });
  const dispose = () => {
    if (installations.get(root)?.dispose !== dispose) return;
    installations.delete(root); observer?.disconnect();
    root.removeEventListener('click', onClick); root.removeEventListener('close', onClose, true);
    for (const dialog of [...owned]) {
      const state = states.get(dialog);
      if (!state) continue;
      disposeState(dialog, state, false);
      if (dialog.open && typeof dialog.close === 'function') { try { dialog.close(); } catch (_) {} }
    }
  };
  installations.set(root, { submitPort, failureLabel, dispose });
  return dispose;
}
