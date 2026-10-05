// Clipboard is browser-local enhancement. The selectable native value is authoritative.
export function install(root = document) {
  const document = root.ownerDocument ?? root;
  const view = document.defaultView;
  const clipboard = view?.navigator?.clipboard;
  if (typeof clipboard?.writeText !== 'function') return () => {};
  const selector = '[data-cui-component="copy-field"]';
  const enhanced = new Set();
  const pending = new WeakMap();
  let active = true;
  const owns = node => node === root || (typeof root.contains === 'function' ? root.contains(node) : node.isConnected);
  function components(scope) {
    return [...(scope.matches?.(selector) ? [scope] : []), ...(scope.querySelectorAll?.(selector) ?? [])];
  }
  function parts(component) {
    const input = component.querySelector('[data-cui-copy-source]');
    const button = component.querySelector('[data-cui-copy-trigger]');
    const status = component.querySelector('[data-cui-copy-status]');
    if (!input || !button || !status || ['idle','copied','failed'].some(key => !button.getAttribute('data-cui-copy-label-'+key))) return null;
    return { input, button, status };
  }
  function enhance(scope) {
    for (const component of components(scope)) {
      const p = parts(component);
      if (p && owns(component)) { p.button.hidden = false; enhanced.add(component); }
    }
  }
  enhance(root);
  const Observer = view?.MutationObserver;
  const observer = Observer ? new Observer(records => {
    for (const record of records) for (const node of record.addedNodes ?? []) enhance(node);
    for (const component of enhanced) if (!owns(component)) enhanced.delete(component);
  }) : null;
  observer?.observe(root, { childList: true, subtree: true });
  async function click(event) {
    const button = event.target?.closest?.('[data-cui-copy-trigger]');
    const component = button?.closest(selector);
    if (!component || !enhanced.has(component) || pending.has(button)) return;
    const p = parts(component);
    if (!p || p.button !== button) return;
    event.preventDefault();
    const value = p.input.value;
    pending.set(button, true);
    button.disabled = true;
    let outcome = 'copied';
    try { await clipboard.writeText(value); } catch { outcome = 'failed'; }
    pending.delete(button);
    if (!active || !button.isConnected || !component.isConnected || !owns(component)) return;
    button.disabled = false;
    // An async response must not claim a later edited/replaced value was copied.
    const current = parts(component);
    if (!current || current.input !== p.input || current.button !== button || current.status !== p.status || p.input.value !== value) { button.setAttribute('aria-label', button.getAttribute('data-cui-copy-label-idle')); if(current)current.status.textContent = ''; return; }
    const label = button.getAttribute('data-cui-copy-label-'+outcome);
    button.setAttribute('aria-label', label);
    p.status.textContent = label;
    if (view?.CustomEvent) component.dispatchEvent(new view.CustomEvent(outcome === 'copied' ? 'cui:copy-succeeded' : 'cui:copy-failed', { bubbles: true }));
  }
  root.addEventListener('click', click);
  return function cleanup() {
    active = false;
    observer?.disconnect();
    root.removeEventListener('click', click);
    for (const component of enhanced) {
      const p = parts(component);
      if (p) { p.button.hidden = true; p.button.disabled = false; p.button.setAttribute('aria-label', p.button.getAttribute('data-cui-copy-label-idle')); p.status.textContent = ''; }
    }
    enhanced.clear();
  };
}
