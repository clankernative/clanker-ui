const COMPONENT = '[data-cui-component="modal"]';
const DIALOG = '[data-cui-modal-dialog]';
const ownedOpeners = new WeakMap();
const installations = new WeakMap();

function closest(target, selector) {
  return target && typeof target.closest === "function" ? target.closest(selector) : null;
}
function inRoot(root, node) {
  return !!node && (node === root || typeof root.contains === "function" && root.contains(node));
}
function partsFor(element) {
  const component = closest(element, COMPONENT);
  if (!component) return null;
  const dialog = component.querySelector(DIALOG);
  const trigger = component.querySelector("[data-cui-modal-trigger]");
  return dialog && trigger ? { component, dialog, trigger } : null;
}

/** Install delegated enhancement for dialogs under root. The link remains the fallback. */
export function install(root = document) {
  const previous = installations.get(root);
  if (previous) return previous;
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
      } catch (_) {
        // Do not cancel navigation if native enhancement fails.
      }
      return;
    }
    const close = closest(event.target, "[data-cui-modal-close]");
    if (close) {
      const parts = partsFor(close);
      if (parts && inRoot(root, close) && parts.dialog.open && typeof parts.dialog.close === "function") {
        try { parts.dialog.close(); } catch (_) { /* retain native state */ }
      }
      return;
    }
    const dialog = closest(event.target, DIALOG);
    if (dialog && inRoot(root, dialog) && closest(dialog, COMPONENT) && event.target === dialog && dialog.open && typeof dialog.close === "function") {
      try { dialog.close(); } catch (_) { /* retain native state */ }
    }
  };
  const onClose = (event) => {
    const dialog = event.target;
    if (!dialog || typeof dialog.matches !== "function" || !dialog.matches(DIALOG)) return;
    const entry = ownedOpeners.get(dialog);
    if (!entry || entry.owner !== dispose) return;
    ownedOpeners.delete(dialog);
    ownedDialogs.delete(dialog);
    if (inRoot(root, entry.trigger) && typeof entry.trigger.focus === "function") entry.trigger.focus();
  };
  root.addEventListener("click", onClick);
  root.addEventListener("close", onClose, true);
  const dispose = () => {
    if (installations.get(root) !== dispose) return;
    installations.delete(root);
    root.removeEventListener("click", onClick);
    root.removeEventListener("close", onClose, true);
    for (const dialog of ownedDialogs) {
      if (ownedOpeners.get(dialog)?.owner !== dispose) continue;
      ownedOpeners.delete(dialog);
      if (dialog.open && typeof dialog.close === "function") {
        try { dialog.close(); } catch (_) { /* teardown is best effort */ }
      }
    }
    ownedDialogs.clear();
  };
  installations.set(root, dispose);
  return dispose;
}
