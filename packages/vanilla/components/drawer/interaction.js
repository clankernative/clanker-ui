const COMPONENT = '[data-cui-component="drawer"]';
const DIALOG = '[data-cui-modal-dialog]';
const ownedOpeners = new WeakMap();
const installations = new WeakMap();
const closest = (target, selector) => target && typeof target.closest === "function" ? target.closest(selector) : null;
const inRoot = (root, node) => !!node && (node === root || typeof root.contains === "function" && root.contains(node));
function partsFor(element) {
  const component = closest(element, COMPONENT);
  if (!component) return null;
  const dialog = component.querySelector(DIALOG);
  const trigger = component.querySelector("[data-cui-modal-trigger]");
  return dialog && trigger ? { dialog, trigger } : null;
}
/** Install delegated enhancement for drawers. The safe link is never removed as fallback. */
export function install(root = document) {
  const previous = installations.get(root); if (previous) return previous;
  const ownedDialogs = new Set();
  const onClick = (event) => {
    const trigger = closest(event.target, "[data-cui-modal-trigger]");
    if (trigger) {
      if (!inRoot(root, trigger)) return;
      const parts = partsFor(trigger);
      if (!parts || typeof parts.dialog.showModal !== "function") return;
      try {
        if (!parts.dialog.open) {
          parts.dialog.showModal();
          if (!parts.dialog.open) return;
          ownedOpeners.set(parts.dialog, { owner: dispose, trigger });
          ownedDialogs.add(parts.dialog);
        }
        event.preventDefault();
      } catch (_) { /* failed enhancement follows the real href */ }
      return;
    }
    const close = closest(event.target, "[data-cui-modal-close]");
    if (close) { const parts = partsFor(close); if (parts && inRoot(root, close) && parts.dialog.open && typeof parts.dialog.close === "function") try { parts.dialog.close(); } catch (_) {} return; }
    const dialog = closest(event.target, DIALOG);
    if (dialog && inRoot(root, dialog) && event.target === dialog && dialog.open && typeof dialog.close === "function") try { dialog.close(); } catch (_) {}
  };
  const onClose = (event) => {
    const dialog = event.target;
    if (!dialog || typeof dialog.matches !== "function" || !dialog.matches(DIALOG)) return;
    const entry = ownedOpeners.get(dialog);
    if (!entry || entry.owner !== dispose) return;
    ownedOpeners.delete(dialog); ownedDialogs.delete(dialog);
    if (inRoot(root, entry.trigger) && typeof entry.trigger.focus === "function") entry.trigger.focus();
  };
  root.addEventListener("click", onClick); root.addEventListener("close", onClose, true);
  const dispose = () => {
    if (installations.get(root) !== dispose) return;
    installations.delete(root); root.removeEventListener("click", onClick); root.removeEventListener("close", onClose, true);
    for (const dialog of ownedDialogs) {
      if (ownedOpeners.get(dialog)?.owner !== dispose) continue;
      ownedOpeners.delete(dialog);
      if (dialog.open && typeof dialog.close === "function") try { dialog.close(); } catch (_) {}
    }
    ownedDialogs.clear();
  };
  installations.set(root, dispose); return dispose;
}
