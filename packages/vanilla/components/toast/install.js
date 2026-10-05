const REGION = '[data-cui-component="toast"]';
const TOAST = "[data-cui-toast]";

/** Install delegated toast dismissal, pausable timers, and BFCache restoration. */
export function install(root = document) {
  const doc = root.ownerDocument ?? (root.nodeType === 9 ? root : globalThis.document);
  const view = doc?.defaultView ?? globalThis.window;
  const ElementType = view?.Element ?? globalThis.Element;
  const NodeType = view?.Node ?? globalThis.Node;
  const AbortControllerType = view?.AbortController ?? globalThis.AbortController;
  const Observer = view?.MutationObserver ?? globalThis.MutationObserver;
  const regions = new Set();
  const states = new Map();
  const snapshots = new Map();
  const listeners = new AbortControllerType();
  const now = () => view?.performance?.now?.() ?? globalThis.performance?.now?.() ?? Date.now();
  const setTimer = view?.setTimeout?.bind(view) ?? globalThis.setTimeout;
  const clearTimer = view?.clearTimeout?.bind(view) ?? globalThis.clearTimeout;
  const within = (node, parent) => node === parent || parent.contains?.(node);
  const clear = (toast) => { const state = states.get(toast); if (state?.timer !== null && state?.timer !== undefined) clearTimer(state.timer); states.delete(toast); };
  const pause = (toast) => { const state = states.get(toast); if (!state || state.timer === null) return; clearTimer(state.timer); state.remaining = Math.max(0, state.remaining - (now() - state.started)); state.timer = null; };
  const dismiss = (toast) => { clear(toast); toast.dataset.cuiToastLeaving = ""; toast.remove(); };
  const resume = (toast) => { const state = states.get(toast); if (!state || state.timer !== null || doc?.hidden || toast.matches(":hover") || toast.contains(doc?.activeElement)) return; state.started = now(); state.timer = setTimer(() => dismiss(toast), state.remaining); };
  const schedule = (toast) => {
    if (states.has(toast) || !toast.hasAttribute("data-cui-toast-timeout")) return;
    const duration = Number(toast.getAttribute("data-cui-toast-timeout"));
    if (!Number.isInteger(duration) || duration < 1000 || duration > 60000) return;
    toast.style.setProperty("--cui-toast-timeout", `${duration}ms`);
    states.set(toast, { remaining: duration, started: now(), timer: null }); resume(toast);
  };
  const forgetTree = (node) => {
    for (const toast of states.keys()) if (within(toast, node)) clear(toast);
    for (const region of [...regions]) if (within(region, node)) { regions.delete(region); snapshots.delete(region); }
  };
  const scanToasts = (node) => {
    if (node.matches?.(TOAST)) schedule(node);
    node.querySelectorAll?.(TOAST).forEach(schedule);
  };
  const attachRegion = (region) => {
    if (regions.has(region)) return;
    regions.add(region);
    snapshots.set(region, [...region.childNodes].map((node) => node.cloneNode(true)));
    region.querySelectorAll(TOAST).forEach(schedule);
  };
  const scanRegions = (node) => {
    if (node.matches?.(REGION)) attachRegion(node);
    node.querySelectorAll?.(REGION).forEach(attachRegion);
  };
  const initialRegions = [...(root.matches?.(REGION) ? [root] : []), ...root.querySelectorAll(REGION)];
  initialRegions.forEach(attachRegion);
  const observer = typeof Observer === "function" ? new Observer((records) => {
    for (const record of records) {
      for (const node of record.removedNodes) forgetTree(node);
      for (const node of record.addedNodes) { scanRegions(node); scanToasts(node); }
    }
  }) : null;
  observer?.observe(root, { childList: true, subtree: true });

  const element = (target) => ElementType && target instanceof ElementType ? target : null;
  const node = (target) => NodeType && target instanceof NodeType ? target : null;
  const containing = (target) => element(target)?.closest(TOAST) ?? null;
  const click = (event) => { const button = element(event.target)?.closest("[data-cui-toast-dismiss]"); const toast = containing(button); if (toast && [...regions].some((region) => region.contains(toast))) dismiss(toast); };
  const over = (event) => { const toast = containing(event.target); if (toast) pause(toast); };
  const out = (event) => { const toast = containing(event.target); if (toast && !(node(event.relatedTarget) && toast.contains(event.relatedTarget))) resume(toast); };
  root.addEventListener("click", click, { signal: listeners.signal });
  root.addEventListener("pointerover", over, { signal: listeners.signal }); root.addEventListener("pointerout", out, { signal: listeners.signal });
  root.addEventListener("focusin", over, { signal: listeners.signal }); root.addEventListener("focusout", out, { signal: listeners.signal });
  const visibility = () => { for (const toast of states.keys()) doc?.hidden ? pause(toast) : resume(toast); };
  doc?.addEventListener("visibilitychange", visibility, { signal: listeners.signal });
  const restored = () => {
    for (const region of regions) {
      const policy = region.getAttribute("data-cui-history-policy") || "reset";
      if (policy === "preserve") continue;
      for (const toast of region.querySelectorAll(TOAST)) clear(toast);
      if (policy === "remove") region.replaceChildren();
      else { region.replaceChildren(...(snapshots.get(region) ?? []).map((node) => node.cloneNode(true))); region.querySelectorAll(TOAST).forEach(schedule); }
    }
  };
  view?.addEventListener("pageshow", (event) => { if (event.persisted) restored(); }, { signal: listeners.signal });
  return () => { listeners.abort(); observer?.disconnect(); for (const toast of [...states.keys()]) clear(toast); regions.clear(); snapshots.clear(); };
}
