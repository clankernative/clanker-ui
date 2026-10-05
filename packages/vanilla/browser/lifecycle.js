// Presentation lifecycle only. Installers are explicit admitted module functions,
// never names/configuration received from an application operation or server model.
const installations = new WeakMap();

export function install(root, installers = []) {
  if (!root?.addEventListener || installers.some(value => typeof value !== 'function')) {
    throw new TypeError('A lifecycle root and explicit component installers are required.');
  }
  const existing = installations.get(root);
  if (existing) {
    if (existing.installers.length !== installers.length || existing.installers.some((value,index) => value !== installers[index])) {
      throw new Error('This root already owns a different component installation.');
    }
    return existing.stop;
  }
  const document = root.ownerDocument ?? root;
  const view = document.defaultView;
  const cleanups = [];
  let stopped = false;
  function observe(name, outcome) {
    if (view?.CustomEvent) root.dispatchEvent(new view.CustomEvent('cui:observation', {
      bubbles: true, detail: Object.freeze({schemaVersion:1, scope:'lifecycle', name, outcome})
    }));
  }
  function stop() {
    if (stopped) return;
    stopped = true;
    view?.removeEventListener('pagehide', hide);
    view?.removeEventListener('pageshow', show);
    installations.delete(root);
    let first;
    for (const cleanup of cleanups.reverse()) {
      try { cleanup(); } catch (error) { first ??= error; }
    }
    observe('runtime.stop', first ? 'error' : 'ok');
    if (first) throw first;
  }
  function hide(event) {
    observe('page.suspended', 'ok');
    // Keep owned timers/history policies alive through BFCache. Each component
    // pauses on visibility/pagehide and handles its own persisted restoration.
    if (!event.persisted) stop();
  }
  function show() { observe('page.restored', 'ok'); }
  try {
    for (const start of installers) {
      const cleanup = start(root);
      if (typeof cleanup !== 'function') throw new TypeError('A component installer must return cleanup.');
      cleanups.push(cleanup);
    }
    view?.addEventListener('pagehide', hide);
    view?.addEventListener('pageshow', show);
    installations.set(root, {stop, installers:installers.slice()});
    observe('runtime.install', 'ok');
    return stop;
  } catch (error) {
    try { stop(); } catch { /* Preserve the original installation error. */ }
    observe('runtime.install', 'error');
    throw error;
  }
}

// Values are bounded browser-local offsets and stable IDs, not record data.
export function captureScrollPosition(owner) {
  const bounds = owner.getBoundingClientRect();
  const anchor = [...owner.querySelectorAll('[data-cui-scroll-anchor][id], [data-cui-patch-item][id]')]
    .find(element => { const r=element.getBoundingClientRect(); return r.bottom > bounds.top && r.top < bounds.bottom; });
  const focus = owner.ownerDocument.activeElement;
  return {
    top: owner.scrollTop, left: owner.scrollLeft,
    anchor: anchor ? {id:anchor.id, offset:anchor.getBoundingClientRect().top-bounds.top} : null,
    focus: focus?.id && owner.contains(focus) ? focus.id : null,
  };
}
export function restoreScrollPosition(owner, snapshot) {
  if (!snapshot || !Number.isFinite(snapshot.top) || !Number.isFinite(snapshot.left)) return;
  owner.scrollTop = Math.max(0, snapshot.top);
  owner.scrollLeft = snapshot.left; // RTL scroll containers may use negative offsets.
  const document = owner.ownerDocument;
  const anchor = snapshot.anchor && Number.isFinite(snapshot.anchor.offset) ? document.getElementById(snapshot.anchor.id) : null;
  if (anchor && owner.contains(anchor)) {
    owner.scrollTop += anchor.getBoundingClientRect().top-owner.getBoundingClientRect().top-snapshot.anchor.offset;
  }
  const focused = snapshot.focus ? document.getElementById(snapshot.focus) : null;
  if (focused && owner.contains(focused) && (!document.activeElement || document.activeElement === document.body)) {
    focused.focus({preventScroll:true});
  }
}
export function preserveScroll(owner, update) {
  const snapshot = captureScrollPosition(owner);
  // The host/app supplies its own admitted update. This helper neither reads
  // HTML strings nor selects a route, operation, or transport.
  let continued = false;
  const mark = () => { continued = true; };
  const events = ['wheel','touchmove','pointerdown','keydown'];
  for (const name of events) owner.addEventListener(name,mark,{capture:true,passive:true});
  const cleanup = () => { for (const name of events) owner.removeEventListener(name,mark,{capture:true}); };
  function finish(value) {
    cleanup();
    if (!continued && owner.isConnected !== false) restoreScrollPosition(owner,snapshot);
    return value;
  }
  try {
    const result = update();
    if (result && typeof result.then === 'function') {
      return Promise.resolve(result).then(finish,error => {cleanup();throw error;});
    }
    return finish(result);
  } catch (error) {cleanup();throw error;}
}
