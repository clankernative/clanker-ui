const SELECTOR = '[data-cui-component="number"]';
const installations = new WeakMap();
const leases = new WeakMap();
const MOTION = { maxGlyphs: 64, maxDuration: 1000, easing: 'cubic-bezier(.2,.8,.2,1)' };

// On update: publish canonical text immediately, roll old/new digit pairs together,
// then remove the decorative layer. Initial renders and incompatible labels snap.
export function install(root = document) {
  const prior = installations.get(root);
  if (prior) return prior;
  const doc = root.ownerDocument ?? root;
  const view = doc.defaultView;
  const reduced = view?.matchMedia?.('(prefers-reduced-motion: reduce)');
  const owner = {};
  const owned = new Set();
  const active = new Map();
  let stopped = false;
  const contains = node => node === root || !!root.contains?.(node);
  const elements = scope => [
    ...(scope.matches?.(SELECTOR) ? [scope] : []),
    ...(scope.querySelectorAll?.(SELECTOR) ?? []),
  ];

  function settle(el, label = el.getAttribute('data-cui-number-value')) {
    const state = active.get(el);
    if (state) {
      active.delete(el);
      for (const animation of state.animations) animation.cancel();
      state.layer.remove();
      if (state.canonical.style.opacity === '0') {
        if (state.opacity) state.canonical.style.opacity = state.opacity;
        else state.canonical.style.removeProperty('opacity');
      }
    }
    const canonical = el.querySelector('.cui-number__canonical');
    if (canonical && label != null) canonical.textContent = label;
  }

  function release(el) {
    if (leases.get(el) !== owner) return;
    settle(el);
    leases.delete(el);
    owned.delete(el);
  }

  function scan(scope) {
    for (const el of elements(scope)) {
      if (!contains(el) || leases.has(el)) continue;
      leases.set(el, owner);
      owned.add(el);
      settle(el);
    }
  }

  function roll(el) {
    if (stopped || !contains(el) || !el.matches?.(SELECTOR)) return;
    if (!leases.has(el)) { scan(el); return; }
    if (leases.get(el) !== owner) return;
    const label = el.getAttribute('data-cui-number-value');
    const canonical = el.querySelector('.cui-number__canonical');
    if (label == null || !canonical) { settle(el); return; }
    const old = canonical.textContent;
    settle(el, label);
    const duration = Number(el.getAttribute('data-cui-number-duration'));
    if (el.getAttribute('data-cui-number-motion') !== 'roll' || reduced?.matches || doc.hidden ||
        el.isConnected === false || !Number.isInteger(duration) || duration <= 0 || duration > MOTION.maxDuration ||
        !view?.Element?.prototype?.animate || !old || old.length > MOTION.maxGlyphs || label.length > MOTION.maxGlyphs ||
        !/^[\x20-\x7e]*$/.test(old) || !/^[\x20-\x7e]*$/.test(label) ||
        view?.getComputedStyle?.(el)?.direction === 'rtl') return;
    if (old.length !== label.length) {
      if (shapeChangeAllowed(old, label)) transitionWholeLabel(el, old, label, duration);
      return;
    }
    const changed = [];
    for (let i = 0; i < label.length; i++) {
      if (old[i] === label[i]) continue;
      if (!/^[0-9]$/.test(old[i]) || !/^[0-9]$/.test(label[i])) {
        if (suffixChangeAllowed(old, label)) transitionWholeLabel(el, old, label, duration);
        return;
      }
      changed.push(i);
    }
    if (!changed.length) return;

    const layer = doc.createElement('span');
    layer.className = 'cui-number__roll';
    layer.setAttribute('aria-hidden', 'true');
    const pairs = [];
    for (let i = 0; i < label.length; i++) {
      const glyph = doc.createElement('span');
      if (changed.includes(i)) {
        glyph.className = 'cui-number__column';
        const outgoing = doc.createElement('span');
        const incoming = doc.createElement('span');
        outgoing.textContent = old[i];
        incoming.textContent = label[i];
        glyph.append(outgoing, incoming);
        pairs.push([outgoing, incoming]);
      } else glyph.textContent = label[i];
      layer.append(glyph);
    }
    const current = { layer, canonical, opacity: canonical.style.opacity, animations: [] };
    active.set(el, current);
    el.append(layer);
    canonical.style.opacity = '0';
    const finished = [];
    try {
      for (const [outgoing, incoming] of pairs) {
        const upward = el.getAttribute('data-cui-number-direction') !== 'down';
        for (const [glyph, from, to] of (upward ? [[outgoing, '0%', '-100%'], [incoming, '100%', '0%']] : [[outgoing, '0%', '100%'], [incoming, '-100%', '0%']])) {
          const animation = glyph.animate(
            [{ transform: `translateY(${from})` }, { transform: `translateY(${to})` }],
            { duration, easing: MOTION.easing, fill: 'both' },
          );
          current.animations.push(animation);
          finished.push(animation.finished.catch(() => undefined));
        }
      }
    } catch { settle(el); return; }
    Promise.all(finished).then(() => {
      if (active.get(el) === current) settle(el);
    });
  }

  function shapeChangeAllowed(old, label) {
    return /^[0-9][0-9,. ]*$/.test(old) && /^[0-9][0-9,. ]*$/.test(label);
  }

  function suffixChangeAllowed(old, label) {
    const before = old.match(/^([0-9][0-9,. ]*)([A-Za-z]{1,4})$/);
    const after = label.match(/^([0-9][0-9,. ]*)([A-Za-z]{1,4})$/);
    return !!before && !!after && before[1].replace(/[0-9]/g, '#') === after[1].replace(/[0-9]/g, '#') && before[2] !== after[2];
  }

  function transitionWholeLabel(el, old, label, duration) {
    const canonical = el.querySelector('.cui-number__canonical');
    if (!canonical) return;
    const layer = doc.createElement('span');
    layer.className = 'cui-number__roll cui-number__whole';
    layer.setAttribute('aria-hidden', 'true');
    layer.textContent = old;
    const current = { layer, canonical, opacity: canonical.style.opacity, animations: [] };
    active.set(el, current);
    el.append(layer);
    canonical.style.opacity = '0';
    try {
      const outgoing = layer.animate([{ opacity: 1, transform: 'translateY(0%)' }, { opacity: 0, transform: 'translateY(-18%)' }], { duration, easing: MOTION.easing, fill: 'both' });
      const incoming = canonical.animate([{ opacity: 0, transform: 'translateY(18%)' }, { opacity: 1, transform: 'translateY(0%)' }], { duration, easing: MOTION.easing, fill: 'both' });
      current.animations.push(outgoing, incoming);
      Promise.all([outgoing.finished.catch(() => undefined), incoming.finished.catch(() => undefined)]).then(() => {
        if (active.get(el) === current) settle(el, label);
      });
    } catch { settle(el, label); }
  }

  function beforePatch(event) {
    const { current, incoming } = event.detail ?? {};
    if (!current || !incoming || event.target !== current || !contains(current)) return;
    const candidates = new Map(elements(incoming).filter(el => el.id).map(el => [el.id, el]));
    for (const old of [...owned]) {
      if (!current.contains(old) || !old.id) continue;
      const fresh = candidates.get(old.id);
      if (!fresh || fresh.getAttribute('data-cui-component') !== 'number') continue;
      for (const name of ['data-cui-number-value', 'data-cui-number-motion', 'data-cui-number-duration', 'data-cui-number-direction', 'class']) {
        const value = fresh.getAttribute(name);
        if (value == null) old.removeAttribute(name); else old.setAttribute(name, value);
      }
      fresh.replaceWith(old);
    }
  }

  root.addEventListener?.('cui:before-patch', beforePatch);
  scan(root);
  const Observer = view?.MutationObserver;
  const observer = typeof Observer === 'function' ? new Observer(records => {
    for (const record of records) {
      if (record.type === 'attributes') roll(record.target);
      else for (const added of record.addedNodes ?? []) scan(added);
    }
    for (const el of owned) if (!contains(el)) release(el);
  }) : null;
  observer?.observe(root, {
    childList: true, subtree: true, attributes: true, attributeFilter: ['data-cui-number-value'],
  });
  // A root cannot observe its own removal from its parent.
  const lifetime = typeof Observer === 'function' && root !== doc && doc.nodeType === 9 ? new Observer(() => {
    if (!root.isConnected) cleanup();
  }) : null;
  lifetime?.observe(doc, { childList: true, subtree: true });
  const onMotion = () => { if (reduced?.matches) for (const el of active.keys()) settle(el); };
  const onVisibility = () => { if (doc.hidden) for (const el of active.keys()) settle(el); };
  reduced?.addEventListener?.('change', onMotion);
  doc.addEventListener?.('visibilitychange', onVisibility);
  function cleanup() {
    if (stopped) return;
    stopped = true;
    observer?.disconnect();
    lifetime?.disconnect();
    root.removeEventListener?.('cui:before-patch', beforePatch);
    reduced?.removeEventListener?.('change', onMotion);
    doc.removeEventListener?.('visibilitychange', onVisibility);
    for (const el of owned) release(el);
    installations.delete(root);
  }
  installations.set(root, cleanup);
  return cleanup;
}
