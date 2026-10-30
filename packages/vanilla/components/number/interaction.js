const SELECTOR = '[data-cui-component="number"]';
const installations = new WeakMap();
const leases = new WeakMap();
const MOTION = { maxGlyphs: 64, maxDuration: 1000 };
const DIGIT = /^[0-9]$/;
const PRINTABLE_ASCII = /^[\x20-\x7e]*$/;

// Number text is owned by the canonical span; the odometer is only a CSS-driven
// visual projection. Existing digit columns are keyed by distance from the right.
export function install(root = document) {
  const prior = installations.get(root);
  if (prior) return prior;
  const doc = root.ownerDocument ?? root;
  const view = doc.defaultView;
  const reduced = view?.matchMedia?.('(prefers-reduced-motion: reduce)');
  const owner = {};
  const owned = new Set();
  const states = new Map();
  let stopped = false;
  const contains = node => node === root || !!root.contains?.(node);
  const elements = scope => [
    ...(scope.matches?.(SELECTOR) ? [scope] : []),
    ...(scope.querySelectorAll?.(SELECTOR) ?? []),
  ];

  function directionOf(el) {
    return el.getAttribute('data-cui-number-direction') || 'up';
  }

  function durationOf(el) {
    const raw = el.getAttribute('data-cui-number-duration');
    const duration = raw == null ? 320 : Number(raw);
    return Number.isInteger(duration) && duration >= 0 && duration <= MOTION.maxDuration
      ? duration
      : null;
  }

  function parsedLabel(label) {
    const chars = Array.from(label);
    const first = chars.findIndex(char => DIGIT.test(char));
    if (first < 0) return null;
    let last = chars.length - 1;
    while (last >= first && !DIGIT.test(chars[last])) last--;
    const prefix = chars.slice(0, first);
    const core = chars.slice(first, last + 1);
    const suffix = chars.slice(last + 1);
    if (!/^[0-9]+(?:[,. ][0-9]+)*$/.test(core.join(''))) return null;
    return { prefix, core, suffix, digitCount: core.filter(char => DIGIT.test(char)).length };
  }

  function separatorsByDigitsRight(parsed) {
    const separators = new Map();
    let digitsRight = 0;
    for (let index = parsed.core.length - 1; index >= 0; index--) {
      const char = parsed.core[index];
      if (DIGIT.test(char)) digitsRight++;
      else separators.set(digitsRight, char);
    }
    return separators;
  }

  function compatibleShapes(before, after) {
    if (!before || !after || before.prefix.join('') !== after.prefix.join('') ||
        before.suffix.join('') !== after.suffix.join('')) return false;
    const left = separatorsByDigitsRight(before);
    const right = separatorsByDigitsRight(after);
    const sharedDigits = Math.min(before.digitCount, after.digitCount);
    // A separator exactly at the edge of the shared digits belongs to an
    // extra leading group; separators inside the shared part must match.
    for (let digitsRight = 1; digitsRight < sharedDigits; digitsRight++) {
      if (left.get(digitsRight) !== right.get(digitsRight)) return false;
    }
    return true;
  }

  function glyphRecords(label, parsed) {
    if (!parsed) {
      const chars = Array.from(label);
      let rank = 0;
      const records = new Array(chars.length);
      for (let index = chars.length - 1; index >= 0; index--) {
        const char = chars[index];
        if (DIGIT.test(char)) records[index] = { key: `digit:${rank++}`, kind: 'digit', char };
        else records[index] = { key: `text:${index}`, kind: 'text', char };
      }
      return records;
    }

    const core = new Array(parsed.core.length);
    let digitsRight = 0;
    for (let index = parsed.core.length - 1; index >= 0; index--) {
      const char = parsed.core[index];
      if (DIGIT.test(char)) {
        core[index] = { key: `digit:${digitsRight++}`, kind: 'digit', char };
      } else {
        core[index] = { key: `separator:${digitsRight}`, kind: 'text', char };
      }
    }
    return [
      ...parsed.prefix.map((char, index) => ({ key: `prefix:${index}`, kind: 'text', char })),
      ...core,
      ...parsed.suffix.map((char, index) => ({ key: `suffix:${index}`, kind: 'text', char })),
    ];
  }

  function digitIndex(char, direction) {
    const value = Number(char);
    return direction === 'down' ? 9 - value : value;
  }

  function configure(odometer, direction, duration) {
    odometer.style.setProperty('--cui-number-duration', `${duration == null ? 0 : duration}ms`);
  }

  function makeGlyph(record, direction, entering = false) {
    const node = doc.createElement('span');
    if (record.kind === 'digit') {
      node.className = entering
        ? 'cui-number__digit cui-number__digit--entering'
        : 'cui-number__digit';
      node.style.setProperty('--cui-number-digit', String(entering
        ? (direction === 'down' ? 9 : 0)
        : digitIndex(record.char, direction)));
      const sizer = doc.createElement('span');
      sizer.className = 'cui-number__digit-sizer';
      sizer.textContent = '0';
      const strip = doc.createElement('span');
      strip.className = 'cui-number__strip';
      const digits = direction === 'down' ? '9876543210' : '0123456789';
      for (const char of digits) {
        const item = doc.createElement('span');
        item.textContent = char;
        strip.append(item);
      }
      node.append(sizer, strip);
    } else {
      node.className = 'cui-number__glyph';
      node.textContent = record.char;
    }
    return node;
  }

  function makeOdometer(el, label, direction, duration) {
    const odometer = doc.createElement('span');
    odometer.className = 'cui-number__odometer';
    odometer.setAttribute('aria-hidden', 'true');
    configure(odometer, direction, duration);
    const records = glyphRecords(label, parsedLabel(label));
    const glyphs = new Map();
    for (const record of records) {
      const node = makeGlyph(record, direction);
      odometer.append(node);
      glyphs.set(record.key, { node, record });
    }
    const current = el.querySelector('.cui-number__odometer');
    if (current) current.replaceWith(odometer);
    else el.append(odometer);
    return { odometer, glyphs };
  }

  function removeOdometer(state) {
    state.odometer?.remove();
    state.odometer = null;
    state.glyphs = new Map();
  }

  function rebuild(el, state, label, direction, duration) {
    const built = makeOdometer(el, label, direction, duration);
    state.odometer = built.odometer;
    state.glyphs = built.glyphs;
  }

  function updateCompatible(state, label, before, after, direction, duration) {
    const records = glyphRecords(label, after);
    const oldDigitCount = before.digitCount;
    const nextGlyphs = new Map();
    const ordered = [];
    const reusedStrips = [];
    for (const record of records) {
      const previous = state.glyphs.get(record.key);
      const reusable = previous && previous.record.kind === record.kind &&
        (record.kind === 'digit' || previous.record.char === record.char);
      const entering = !reusable && record.kind === 'digit' && Number(record.key.slice(6)) >= oldDigitCount;
      const node = reusable ? previous.node : makeGlyph(record, direction, entering);
      if (reusable && record.kind === 'digit') {
        const strip = node.querySelector('.cui-number__strip');
        if (strip) reusedStrips.push(strip);
      }
      ordered.push({ node, record });
      nextGlyphs.set(record.key, { node, record });
    }

    const wanted = new Set(ordered.map(item => item.node));
    for (const node of [...state.odometer.children]) {
      if (!wanted.has(node)) node.remove();
    }
    let reference = null;
    for (let index = ordered.length - 1; index >= 0; index--) {
      const node = ordered[index].node;
      if (node.parentNode !== state.odometer || node.nextSibling !== reference) {
        state.odometer.insertBefore(node, reference);
      }
      reference = node;
    }

    configure(state.odometer, direction, duration);
    for (const { node, record } of ordered) {
      if (record.kind === 'digit') {
        node.style.setProperty('--cui-number-digit', String(digitIndex(record.char, direction)));
      }
    }
    state.glyphs = nextGlyphs;
    return reusedStrips;
  }

  function invalidateHandoff(state) {
    state.pendingHandoff = null;
  }

  function snapshotStrips(el) {
    const state = states.get(el);
    if (!state?.odometer || typeof view?.getComputedStyle !== 'function') return null;
    const entries = [];
    try {
      for (const { node, record } of state.glyphs.values()) {
        if (record.kind !== 'digit') continue;
        const strip = node.querySelector('.cui-number__strip');
        if (!strip || !state.odometer.contains(strip)) return null;
        const transform = view.getComputedStyle(strip)?.transform;
        if (typeof transform !== 'string' || transform.length === 0) return null;
        entries.push({ strip, transform });
      }
    } catch {
      return null;
    }
    return entries.length ? { odometer: state.odometer, entries } : null;
  }

  function update(el, captured = null) {
    const state = states.get(el);
    if (!state || leases.get(el) !== owner) return null;
    const label = el.getAttribute('data-cui-number-value');
    if (label == null) {
      invalidateHandoff(state);
      return null;
    }

    const oldLabel = state.label;
    const oldMotion = state.motion;
    const oldDirection = state.direction;
    const motion = el.getAttribute('data-cui-number-motion');
    const direction = directionOf(el);
    const duration = durationOf(el);
    if (oldLabel === label && oldMotion === motion && oldDirection === direction && state.duration === duration) {
      return null;
    }
    invalidateHandoff(state);
    const canonical = state.canonical;
    canonical.textContent = label;
    state.label = label;
    state.motion = motion;
    state.direction = direction;
    state.duration = duration;

    if (motion !== 'roll') {
      removeOdometer(state);
      return null;
    }

    if (!state.odometer) {
      rebuild(el, state, label, direction, duration);
      return null;
    }
    if (oldLabel === label && oldMotion === motion && oldDirection === direction) {
      configure(state.odometer, direction, duration);
      return null;
    }

    const before = parsedLabel(oldLabel ?? '');
    const after = parsedLabel(label);
    const asciiAndBounded = PRINTABLE_ASCII.test(oldLabel ?? '') && PRINTABLE_ASCII.test(label) &&
      Array.from(oldLabel ?? '').length <= MOTION.maxGlyphs && Array.from(label).length <= MOTION.maxGlyphs;
    const leftToRight = view?.getComputedStyle?.(el)?.direction !== 'rtl';
    const canRetarget = oldMotion === 'roll' && oldDirection === direction &&
      (direction === 'up' || direction === 'down') && duration != null && duration > 0 &&
      !reduced?.matches && !doc.hidden && el.isConnected !== false && leftToRight &&
      asciiAndBounded && compatibleShapes(before, after);

    if (canRetarget) {
      const reusedStrips = updateCompatible(state, label, before, after, direction, duration);
      const snapshots = new Map((captured?.entries ?? []).map(entry => [entry.strip, entry]));
      const entries = reusedStrips.map(strip => snapshots.get(strip));
      if (captured?.odometer === state.odometer && entries.length > 0 && entries.every(Boolean)) {
        return { odometer: state.odometer, direction, entries };
      }
      return null;
    }
    rebuild(el, state, label, direction, duration);
    return null;
  }

  function visible(el) {
    if (doc.hidden || typeof view?.getComputedStyle !== 'function') return false;
    try {
      for (let node = el; node?.nodeType === 1; node = node.parentElement) {
        const style = view.getComputedStyle(node);
        if (!style || style.display === 'none' || style.visibility === 'hidden' ||
            style.visibility === 'collapse' || style.contentVisibility === 'hidden' || style.opacity === '0') return false;
      }
      if (typeof el.getClientRects === 'function' && el.getClientRects().length === 0) return false;
      return true;
    } catch {
      return false;
    }
  }

  function resumeAfterPatch(el, state, outcome) {
    const handoff = { ...outcome, state };
    state.pendingHandoff = handoff;
    const resume = () => {
      if (state.pendingHandoff !== handoff) return;
      const current = states.get(el) === state && leases.get(el) === owner && !stopped &&
        contains(el) && el.isConnected !== false && state.odometer === handoff.odometer &&
        state.motion === 'roll' && el.getAttribute('data-cui-number-motion') === 'roll' &&
        el.getAttribute('data-cui-number-value') === state.label && state.direction === handoff.direction &&
        directionOf(el) === handoff.direction && durationOf(el) === state.duration && state.duration > 0 &&
        !reduced?.matches && visible(el) && el.contains(handoff.odometer) &&
        handoff.entries.every(entry => handoff.odometer.contains(entry.strip));
      state.pendingHandoff = null;
      if (!current || !handoff.entries.length) return;

      const getStyle = view?.getComputedStyle;
      if (typeof getStyle !== 'function' || handoff.entries.some(({ strip }) =>
        !strip.style || typeof strip.style.getPropertyValue !== 'function' ||
        typeof strip.style.setProperty !== 'function' || typeof strip.style.removeProperty !== 'function')) return;
      const saved = handoff.entries.map(({ strip }) => ({
        strip,
        transform: strip.style.getPropertyValue('transform'),
        transformPriority: strip.style.getPropertyPriority?.('transform') ?? '',
        transition: strip.style.getPropertyValue('transition'),
        transitionPriority: strip.style.getPropertyPriority?.('transition') ?? '',
      }));
      if (saved.some(item => typeof item.strip.style.setProperty !== 'function' ||
          typeof item.strip.style.removeProperty !== 'function' ||
          typeof item.strip.style.getPropertyValue !== 'function')) return;
      try {
        for (const entry of handoff.entries) {
          entry.strip.style.setProperty('transition', 'none', 'important');
          entry.strip.style.setProperty('transform', entry.transform, 'important');
        }
        // One computed-style read flushes all captured starting transforms while connected.
        getStyle.call(view, handoff.entries[0].strip)?.transform;
      } catch {
        // The finally block restores every installer-owned strip even if a style API fails.
      } finally {
        for (const item of saved) {
          if (item.transform) item.strip.style.setProperty('transform', item.transform, item.transformPriority);
          else item.strip.style.removeProperty('transform');
          if (item.transition) item.strip.style.setProperty('transition', item.transition, item.transitionPriority);
          else item.strip.style.removeProperty('transition');
        }
      }
    };
    if (typeof view?.queueMicrotask === 'function') view.queueMicrotask(resume);
    else Promise.resolve().then(resume);
  }

  function release(el) {
    if (leases.get(el) !== owner) return;
    const state = states.get(el);
    if (state) {
      invalidateHandoff(state);
      removeOdometer(state);
      states.delete(el);
    }
    leases.delete(el);
    owned.delete(el);
  }

  function scan(scope) {
    for (const el of elements(scope)) {
      if (!contains(el) || leases.has(el)) continue;
      const canonical = el.querySelector('.cui-number__canonical');
      if (!canonical) continue;
      leases.set(el, owner);
      owned.add(el);
      const label = el.getAttribute('data-cui-number-value') ?? canonical.textContent;
      const state = {
        canonical,
        label,
        motion: el.getAttribute('data-cui-number-motion'),
        direction: directionOf(el),
        duration: durationOf(el),
        odometer: null,
        glyphs: new Map(),
        pendingHandoff: null,
      };
      states.set(el, state);
      canonical.textContent = label;
      if (state.motion === 'roll') rebuild(el, state, label, state.direction, durationOf(el));
    }
  }

  function snapAll() {
    for (const el of owned) {
      const state = states.get(el);
      if (!state) continue;
      invalidateHandoff(state);
      if (state.motion === 'roll') rebuild(el, state, state.label, state.direction, durationOf(el));
      else removeOdometer(state);
    }
  }

  function beforePatch(event) {
    const { current, incoming } = event.detail ?? {};
    if (!current || !incoming || event.target !== current || !contains(current)) return;
    const candidates = new Map(elements(incoming).filter(el => el.id).map(el => [el.id, el]));
    for (const old of [...owned]) {
      if (!current.contains(old) || !old.id) continue;
      const fresh = candidates.get(old.id);
      if (!fresh || fresh.getAttribute('data-cui-component') !== 'number') continue;
      const captured = snapshotStrips(old);
      for (const name of ['data-cui-number-value', 'data-cui-number-motion', 'data-cui-number-duration', 'data-cui-number-direction', 'class']) {
        const value = fresh.getAttribute(name);
        if (value == null) old.removeAttribute(name); else old.setAttribute(name, value);
      }
      const outcome = update(old, captured);
      fresh.replaceWith(old);
      const state = states.get(old);
      if (outcome && state) resumeAfterPatch(old, state, outcome);
    }
  }

  root.addEventListener?.('cui:before-patch', beforePatch);
  scan(root);
  const Observer = view?.MutationObserver;
  const observer = typeof Observer === 'function' ? new Observer(records => {
    for (const record of records) {
      if (record.type === 'attributes') update(record.target);
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
  const onMotion = () => { if (reduced?.matches) snapAll(); };
  const onVisibility = () => { if (doc.hidden) snapAll(); };
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
    for (const el of [...owned]) release(el);
    installations.delete(root);
  }
  installations.set(root, cleanup);
  return cleanup;
}
