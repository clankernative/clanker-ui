const SELECTOR = 'details[data-cui-component="popover"]';
const installations = new WeakMap();
function connected(node, root) { if (!node) return false; if (typeof node.isConnected === "boolean") return node.isConnected; return typeof root.contains === "function" && root.contains(node); }
function detailsFor(target) { return target && typeof target.closest === "function" ? target.closest(SELECTOR) : null; }
function close(popover, restoreFocus, root) {
  popover.open = false;
  if (restoreFocus) {
    const summary = popover.querySelector("summary");
    if (connected(summary, root) && typeof summary.focus === "function") summary.focus();
  }
}
function viewFor(root) { return root.defaultView || (root.ownerDocument && root.ownerDocument.defaultView) || globalThis.window; }
function position(popover, view) {
  const panel = popover.querySelector(".cui-popover__panel"), summary = popover.querySelector("summary");
  if (!panel || !summary || typeof panel.getBoundingClientRect !== "function" || typeof summary.getBoundingClientRect !== "function") return;
  const panelRect = panel.getBoundingClientRect(), triggerRect = summary.getBoundingClientRect();
  const width = Number(view && view.innerWidth) || 0, height = Number(view && view.innerHeight) || 0, gutter = 8, style = panel.style;
  if (!style || typeof style.setProperty !== "function") return;
  const direction = view && typeof view.getComputedStyle === "function" ? view.getComputedStyle(popover).direction : (popover.getAttribute && popover.getAttribute("dir"));
  const rtl = direction === "rtl", alignsEnd = popover.classList && popover.classList.contains("cui-popover--end"), alignRight = alignsEnd !== rtl;
  const desiredLeft = alignRight ? triggerRect.right - panelRect.width : triggerRect.left;
  const nextLeft = Math.max(gutter, Math.min(desiredLeft, Math.max(gutter, width - panelRect.width - gutter)));
  const priorShift = typeof style.getPropertyValue === "function" ? Number.parseFloat(style.getPropertyValue("--cui-popover-shift-x")) || 0 : 0;
  const baseLeft = panelRect.left - priorShift;
  style.setProperty("--cui-popover-shift-x", `${nextLeft - baseLeft}px`);
  const availableAbove = triggerRect.top - gutter, availableBelow = height - triggerRect.bottom - gutter;
  const below = height === 0 || availableBelow >= panelRect.height || availableBelow >= availableAbove;
  if (popover.setAttribute) popover.setAttribute("data-cui-popover-side", below ? "below" : "above");
}
/** Install optional disclosure dismissal and viewport collision positioning under root. */
export function install(root = document) {
  const previous = installations.get(root); if (previous) return previous;
  const view = viewFor(root), positioned = new Map();
  const openPopovers = () => Array.from(root.querySelectorAll ? root.querySelectorAll(`${SELECTOR}[open]`) : []);
  const onPointer = (event) => { for (const popover of openPopovers()) if (!popover.contains(event.target)) close(popover, false, root); };
  const onKeydown = (event) => { const popover = detailsFor(event.target); if (!popover || !popover.open || event.key !== "Escape") return; event.preventDefault(); close(popover, true, root); };
  const onToggle = (event) => {
    const popover = detailsFor(event.target); if (!popover || !popover.open) return;
    const panel = popover.querySelector(".cui-popover__panel");
    if (panel?.style && !positioned.has(panel)) positioned.set(panel, { shift: panel.style.getPropertyValue?.("--cui-popover-shift-x") || "", priority: panel.style.getPropertyPriority?.("--cui-popover-shift-x") || "", side: popover.getAttribute?.("data-cui-popover-side") });
    position(popover, view);
  };
  const onViewportChange = () => { for (const popover of openPopovers()) position(popover, view); };
  root.addEventListener("pointerdown", onPointer); root.addEventListener("keydown", onKeydown); root.addEventListener("toggle", onToggle, true);
  if (view?.addEventListener) { view.addEventListener("resize", onViewportChange); view.addEventListener("scroll", onViewportChange, true); }
  const dispose = () => {
    if (installations.get(root) !== dispose) return;
    installations.delete(root);
    root.removeEventListener("pointerdown", onPointer); root.removeEventListener("keydown", onKeydown); root.removeEventListener("toggle", onToggle, true);
    if (view?.removeEventListener) { view.removeEventListener("resize", onViewportChange); view.removeEventListener("scroll", onViewportChange, true); }
    for (const [panel, original] of positioned) {
      if (original.shift) panel.style.setProperty("--cui-popover-shift-x", original.shift, original.priority); else panel.style.removeProperty?.("--cui-popover-shift-x");
      const popover = panel.closest?.(SELECTOR);
      if (popover) { if (original.side == null) popover.removeAttribute("data-cui-popover-side"); else popover.setAttribute("data-cui-popover-side", original.side); }
    }
    positioned.clear();
  };
  installations.set(root, dispose); return dispose;
}
