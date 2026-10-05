const ROOT = '[data-cui-component="command-menu"]';
const DIALOG = '[data-cui-command-dialog]';
const openers = new WeakMap();
const installations = new WeakMap();
const editable = (target) => !!target?.closest?.('input,textarea,select,[contenteditable]:not([contenteditable="false"]),[role="textbox"]');
const inRoot = (root, node) => !!node && (node === root || !!root.contains?.(node));
function parts(node) {
  const component = node?.closest?.(ROOT);
  if (!component) return null;
  const dialog = component.querySelector(DIALOG);
  const trigger = component.querySelector('[data-cui-command-trigger]');
  const search = component.querySelector('[data-cui-command-search]');
  return dialog && trigger && search ? { component, dialog, trigger, search } : null;
}
function visibleItems(component) { return [...component.querySelectorAll('[data-cui-command-item]:not([hidden])')]; }
function activate(p, item, lease) {
  for (const candidate of p.component.querySelectorAll('[data-cui-command-item]')) {
    const active = candidate === item;
    lease.writeAttribute(candidate, 'data-cui-command-active', active ? '' : null);
    lease.writeAttribute(candidate, 'aria-selected', String(active));
  }
  lease.writeAttribute(p.search, 'aria-activedescendant', item?.id || null);
  item?.scrollIntoView?.({ block: 'nearest' });
}
function filter(p, lease) {
  const query = p.search.value.trim().toLocaleLowerCase();
  let count = 0;
  for (const item of p.component.querySelectorAll('[data-cui-command-item]')) {
    const show = !query || item.getAttribute('data-cui-command-text').toLocaleLowerCase().includes(query);
    lease.writeProperty(item, 'hidden', !show);
    if (!show) { lease.writeAttribute(item, 'data-cui-command-active', null); lease.writeAttribute(item, 'aria-selected', 'false'); }
    if (show) count++;
  }
  for (const group of p.component.querySelectorAll('[data-cui-command-group]')) lease.writeProperty(group, 'hidden', !group.querySelector('[data-cui-command-item]:not([hidden])'));
  const empty = p.component.querySelector('[data-cui-command-empty]');
  if (empty) lease.writeProperty(empty, 'hidden', count !== 0);
  const first = visibleItems(p.component)[0];
  activate(p, first || null, lease);
}
function show(p, opener, lease) {
  if (!inRoot(lease.root, p.trigger) || typeof p.dialog.showModal !== 'function' || p.dialog.open) return false;
  try {
    p.dialog.showModal();
    if (!p.dialog.open) return false;
    openers.set(p.dialog, { owner: lease.dispose, opener });
    lease.dialogs.add(p.dialog);
    lease.writeProperty(p.search, 'value', '');
    filter(p, lease);
    p.search.focus();
    return true;
  } catch (_) { return false; }
}
function makeLease(root) {
  const writes = new Map();
  const dialogs = new Set();
  function write(node, key, kind, value) {
    let byKey = writes.get(node); if (!byKey) { byKey = new Map(); writes.set(node, byKey); }
    const id = `${kind}:${key}`;
    let record = byKey.get(id);
    if (!record) {
      record = { kind, key, before: kind === 'property' ? node[key] : node.getAttribute(key), after: value };
      byKey.set(id, record);
    } else record.after = value;
    if (kind === 'property') node[key] = value;
    else if (value === null) node.removeAttribute(key);
    else node.setAttribute(key, value);
  }
  const lease = {
    root, dialogs, dispose: null,
    writeProperty(node, key, value) { write(node, key, 'property', value); },
    writeAttribute(node, key, value) { write(node, key, 'attribute', value); },
    restore() {
      for (const [node, byKey] of writes) for (const record of byKey.values()) {
        const current = record.kind === 'property' ? node[record.key] : node.getAttribute(record.key);
        if (current !== record.after) continue;
        if (record.kind === 'property') node[record.key] = record.before;
        else if (record.before === null) node.removeAttribute(record.key);
        else node.setAttribute(record.key, record.before);
      }
      writes.clear();
    },
  };
  return lease;
}
/** Install delegated local filtering/navigation. All destinations remain ordinary anchors. */
export function install(root = document) {
  const previous = installations.get(root); if (previous) return previous;
  const lease = makeLease(root);
  const onClick = (event) => {
    const trigger = event.target?.closest?.('[data-cui-command-trigger]');
    if (trigger) { if (!inRoot(root, trigger)) return; const p = parts(trigger); if (p && show(p, trigger, lease)) event.preventDefault(); return; }
    const close = event.target?.closest?.('[data-cui-command-close]');
    if (close) { const p = parts(close); if (p && inRoot(root, close) && p.dialog.open && typeof p.dialog.close === 'function') p.dialog.close(); return; }
    const item = event.target?.closest?.('[data-cui-command-item]');
    if (item) { const p = parts(item); if (p && inRoot(root, item)) activate(p, item, lease); return; }
    const dialog = event.target?.closest?.(DIALOG);
    if (dialog && inRoot(root, dialog) && event.target === dialog && dialog.open && typeof dialog.close === 'function') dialog.close();
  };
  const onInput = (event) => { const p = parts(event.target); if (p && inRoot(root, event.target) && p.search === event.target) filter(p, lease); };
  const onKeydown = (event) => {
    const p = parts(event.target);
    if (p && inRoot(root, event.target) && p.dialog.open && ['ArrowDown','ArrowUp','Home','End','Enter'].includes(event.key)) {
      const items = visibleItems(p.component);
      if (event.key === 'Enter') {
        const active = p.component.querySelector('[data-cui-command-item][data-cui-command-active]') || items[0];
        if (active) { event.preventDefault(); active.click(); }
      } else if (items.length) {
        event.preventDefault();
        const index = items.indexOf(p.component.querySelector('[data-cui-command-item][data-cui-command-active]'));
        const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
        activate(p, items[next], lease);
      }
      return;
    }
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || editable(event.target) || !(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
    for (const component of root.querySelectorAll?.(`${ROOT}[data-cui-command-shortcut]`) || []) {
      if (!inRoot(root, component) || component.getAttribute('data-cui-command-shortcut').toLocaleLowerCase() !== event.key.toLocaleLowerCase()) continue;
      const trigger = component.querySelector('[data-cui-command-trigger]'); const p = trigger && parts(trigger);
      if (p && show(p, trigger, lease)) { event.preventDefault(); break; }
    }
  };
  const onClose = (event) => {
    const dialog = event.target;
    if (!dialog?.matches?.(DIALOG)) return;
    const entry = openers.get(dialog);
    if (!entry || entry.owner !== dispose) return;
    openers.delete(dialog); lease.dialogs.delete(dialog);
    if (inRoot(root, entry.opener)) entry.opener.focus?.();
  };
  root.addEventListener('click', onClick); root.addEventListener('input', onInput); root.addEventListener('keydown', onKeydown); root.addEventListener('close', onClose, true);
  const dispose = () => {
    if (installations.get(root) !== dispose) return;
    installations.delete(root);
    root.removeEventListener('click', onClick); root.removeEventListener('input', onInput); root.removeEventListener('keydown', onKeydown); root.removeEventListener('close', onClose, true);
    for (const dialog of lease.dialogs) {
      if (openers.get(dialog)?.owner !== dispose) continue;
      openers.delete(dialog);
      if (dialog.open && typeof dialog.close === 'function') try { dialog.close(); } catch (_) {}
    }
    lease.dialogs.clear(); lease.restore();
  };
  lease.dispose = dispose;
  installations.set(root, dispose); return dispose;
}
