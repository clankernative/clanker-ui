import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { install } from '../../packages/vanilla/components/number/interaction.js';

// Deterministic DOM/WAAPI substitutes; these are not real-browser qualification.
class FakeElement {
  constructor(tag = 'span') {
    this.tagName = tag;
    this.attrs = new Map();
    this.children = [];
    this.parent = null;
    this.style = { opacity: '', removeProperty(key) { this[key] = ''; } };
    this.nodeType = 1;
    this.animations = [];
    this.listeners = new Map();
  }
  matches(selector) {
    if (selector.startsWith('.')) return (this.className ?? '').split(/\s+/).includes(selector.slice(1));
    return selector === '[data-cui-component="number"]' && this.getAttribute('data-cui-component') === 'number';
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  querySelectorAll(selector) {
    return this.children.flatMap(child => [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)]);
  }
  append(...children) {
    for (const child of children) { child.remove(); child.parent = this; this.children.push(child); }
  }
  replaceChildren(...children) { for (const child of [...this.children]) child.remove(); this.append(...children); }
  replaceWith(node) { if (!this.parent) return; const parent = this.parent, index = parent.children.indexOf(this); this.remove(); node.remove(); node.parent = parent; parent.children.splice(index, 0, node); }
  removeAttribute(key) { this.attrs.delete(key); }
  addEventListener(name, fn) { this.listeners.set(name, fn); }
  removeEventListener(name) { this.listeners.delete(name); }
  dispatchEvent(event) {
    event.target = this;
    for (let node = this; node; node = event.bubbles ? node.parent : null) node.listeners.get(event.type)?.(event);
  }
  get id() { return this.getAttribute('id') ?? ''; }
  set id(value) { this.setAttribute('id', value); }
  remove() {
    if (this.parent) this.parent.children = this.parent.children.filter(child => child !== this);
    this.parent = null;
  }
  setAttribute(key, value) { this.attrs.set(key, String(value)); }
  getAttribute(key) { return this.attrs.get(key) ?? null; }
  contains(node) { return this === node || this.children.some(child => child.contains(node)); }
  get isConnected() { return this.connected === true || (this.parent?.isConnected ?? false); }
  animate(keyframes, options) {
    let resolve, reject;
    const finished = new Promise((a, b) => { resolve = a; reject = b; });
    const animation = {
      keyframes, options, finished, cancelled: false, finish: resolve,
      cancel() { this.cancelled = true; reject(new Error('animation cancelled')); },
    };
    this.animations.push(animation);
    return animation;
  }
  get textContent() { return this._text ?? this.children.map(child => child.textContent).join(''); }
  set textContent(value) { this._text = String(value); this.children = []; }
}
class FakeObserver {
  static instances = [];
  constructor(callback) { this.callback = callback; FakeObserver.instances.push(this); }
  observe(target, options) { this.target = target; this.options = options; }
  disconnect() { this.disconnected = true; }
  fire(records) { if (!this.disconnected) this.callback(records); }
}
function emit(root, records) {
  for (const observer of FakeObserver.instances) if (observer.target === root) observer.fire(records);
}
function makeNumber(value) {
  const number = new FakeElement();
  for (const [key, val] of Object.entries({
    'data-cui-component': 'number', 'data-cui-number-motion': 'roll',
    'data-cui-number-duration': '320', 'data-cui-number-value': value,
  })) number.setAttribute(key, val);
  const canonical = new FakeElement();
  canonical.className = 'cui-number__canonical';
  canonical.textContent = value;
  number.append(canonical);
  return { number, canonical };
}
function setup(value = '12', reducedMotion = false) {
  const media = {
    matches: reducedMotion, listeners: new Set(),
    addEventListener(_name, fn) { this.listeners.add(fn); },
    removeEventListener(_name, fn) { this.listeners.delete(fn); },
  };
  const view = { Element: FakeElement, MutationObserver: FakeObserver, matchMedia: () => media };
  const doc = {
    nodeType: 9, defaultView: view, hidden: false, listeners: new Map(),
    addEventListener(name, fn) { this.listeners.set(name, fn); },
    removeEventListener(name) { this.listeners.delete(name); },
    createElement(tag) { const el = new FakeElement(tag); el.ownerDocument = doc; return el; },
    documentElement: new FakeElement('html'),
  };
  doc.documentElement.connected = true;
  const root = new FakeElement('root');
  root.ownerDocument = doc;
  doc.documentElement.append(root);
  const { number, canonical } = makeNumber(value);
  number.ownerDocument = doc;
  root.append(number);
  const update = label => {
    number.setAttribute('data-cui-number-value', label);
    emit(root, [{ type: 'attributes', target: number }]);
  };
  return { root, number, canonical, doc, media, update };
}
const layer = number => number.querySelector('.cui-number__roll');
const animations = number => layer(number)?.querySelectorAll('.cui-number__column').flatMap(column => column.children.flatMap(glyph => glyph.animations)) ?? [];
const flush = () => new Promise(setImmediate);

test('first paint is static; changed digits roll old-to-new in clipped pairs without changing canonical meaning', () => {
  const x = setup('$1,204.50');
  const cleanup = install(x.root);
  assert.equal(layer(x.number), null);
  x.update('$1,205.50');
  assert.equal(x.canonical.textContent, '$1,205.50');
  assert.equal(x.canonical.getAttribute('aria-hidden'), null);
  assert.equal(layer(x.number).getAttribute('aria-hidden'), 'true');
  const columns = layer(x.number).querySelectorAll('.cui-number__column');
  assert.equal(columns.length, 1);
  assert.deepEqual(columns[0].children.map(glyph => glyph.textContent), ['4', '5']);
  const [outgoing, incoming] = animations(x.number);
  assert.deepEqual(outgoing.keyframes, [{ transform: 'translateY(0%)' }, { transform: 'translateY(-100%)' }]);
  assert.deepEqual(incoming.keyframes, [{ transform: 'translateY(100%)' }, { transform: 'translateY(0%)' }]);
  assert.equal(incoming.options.duration, 320);
  assert.deepEqual(outgoing.options, incoming.options);
  cleanup();
  assert.equal(layer(x.number), null);
  assert.equal(x.canonical.style.opacity, '');
});

test('completion removes decoration and interruption cannot restore a superseded label', async () => {
  const x = setup();
  const cleanup = install(x.root);
  x.update('13');
  const first = animations(x.number);
  x.update('14');
  assert.ok(first.every(animation => animation.cancelled));
  for (const animation of first) animation.finish();
  await flush();
  assert.equal(x.canonical.textContent, '14');
  assert.equal(x.number.children.length, 2);
  for (const animation of animations(x.number)) animation.finish();
  await flush();
  assert.equal(layer(x.number), null);
  assert.equal(x.canonical.textContent, '14');
  assert.equal(x.canonical.style.opacity, '');
  cleanup();
});

test('sign, grouping, separators, suffixes, unsupported glyphs, and long labels snap intact', () => {
  for (const [before, after] of [
    ['-12', '+13'], ['1.2', '1,3'], ['€12', '€13'],
    ['42', '7'.repeat(65)], ['1'.repeat(65), '2'.repeat(65)], ['12', '12'],
  ]) {
    const x = setup(before), cleanup = install(x.root);
    x.update(after);
    assert.equal(x.canonical.textContent, after);
    assert.equal(layer(x.number), null, `${before} -> ${after} should snap`);
    cleanup();
  }
});

test('zero or invalid duration, disabled motion, RTL, absent APIs, and hidden pages stay canonical', () => {
  for (const configure of [
    ...['0', '-1', '1.5', '1001', 'invalid'].map(value => x => x.number.setAttribute('data-cui-number-duration', value)),
    x => x.number.setAttribute('data-cui-number-motion', 'none'),
    x => { x.doc.defaultView.getComputedStyle = () => ({ direction: 'rtl' }); },
    x => { x.doc.defaultView.Element = class {}; },
    x => { x.doc.hidden = true; },
  ]) {
    const x = setup(), cleanup = install(x.root);
    configure(x);
    x.update('13');
    assert.equal(x.canonical.textContent, '13');
    assert.equal(layer(x.number), null);
    cleanup();
  }
});

test('reduced motion and visibility changes cancel active animation immediately', () => {
  const reduced = setup('12', true), disposeReduced = install(reduced.root);
  reduced.update('13');
  assert.equal(layer(reduced.number), null);
  assert.equal(reduced.canonical.textContent, '13');
  disposeReduced();
  for (const reason of ['reduced', 'hidden']) {
    const x = setup(), cleanup = install(x.root);
    x.update('13');
    const running = animations(x.number);
    if (reason === 'reduced') {
      x.media.matches = true;
      for (const listener of x.media.listeners) listener();
    } else {
      x.doc.hidden = true;
      x.doc.listeners.get('visibilitychange')();
    }
    assert.ok(running.every(animation => animation.cancelled));
    assert.equal(layer(x.number), null);
    assert.equal(x.canonical.textContent, '13');
    cleanup();
  }
});

test('removing a whole installer root cancels animation and releases observers and listeners', () => {
  for (const installOnNumber of [false, true]) {
    const x = setup(), root = installOnNumber ? x.number : x.root;
    const cleanup = install(root);
    x.number.setAttribute('data-cui-number-value', '13');
    emit(root, [{ type: 'attributes', target: x.number }]);
    const running = animations(x.number);
    assert.equal(running.length, 2);
    root.remove();
    // The removed root receives no child-list notification of its own removal.
    emit(x.doc, [{ type: 'childList', removedNodes: [root] }]);
    assert.ok(running.every(animation => animation.cancelled));
    assert.equal(layer(x.number), null);
    assert.equal(x.media.listeners.size, 0);
    assert.equal(x.doc.listeners.size, 0);
    assert.ok(FakeObserver.instances.filter(observer => observer.target === root).every(observer => observer.disconnected));
    cleanup();
  }
});

test('before-patch preserves matching number node identity and animates the accepted value', () => {
  const x = setup('12'), cleanup = install(x.root);
  const original = x.number;
  original.id = 'stable-total';
  const currentRegion = new FakeElement('section');
  currentRegion.ownerDocument = x.doc;
  x.root.replaceChildren(currentRegion);
  currentRegion.append(original);
  const incomingRegion = new FakeElement('section');
  const fresh = makeNumber('13');
  fresh.number.id = 'stable-total';
  fresh.number.setAttribute('data-cui-operation', 'not-admitted');
  fresh.number.setAttribute('data-cui-number-evil', 'not-admitted');
  incomingRegion.append(fresh.number);
  currentRegion.dispatchEvent({ type: 'cui:before-patch', bubbles: true, detail: { current: currentRegion, incoming: incomingRegion } });
  assert.equal(incomingRegion.children[0], original);
  assert.equal(original.id, 'stable-total');
  assert.equal(original.getAttribute('data-cui-number-value'), '13');
  currentRegion.replaceChildren(...incomingRegion.children);
  assert.equal(original.getAttribute('data-cui-operation'), null);
  assert.equal(original.getAttribute('data-cui-number-evil'), null);
  emit(x.root, [{ type: 'attributes', target: original }]);
  assert.equal(x.canonical.textContent, '13');
  assert.equal(animations(original).length, 2);
  cleanup();
  assert.equal(x.root.listeners.size, 0);
});

test('shape and suffix changes use bounded whole-label transition; direction is explicit', () => {
  for (const [before, after] of [['999', '1,000'], ['4.2K', '4.3M']]) {
    const x = setup(before), cleanup = install(x.root);
    x.update(after);
    assert.equal(x.canonical.textContent, after);
    assert.equal(layer(x.number).className, 'cui-number__roll cui-number__whole');
    assert.equal(layer(x.number).getAttribute('aria-hidden'), 'true');
    cleanup();
  }
  const x = setup('12'), cleanup = install(x.root);
  x.number.setAttribute('data-cui-number-direction', 'down');
  x.update('11');
  assert.deepEqual(animations(x.number).map(a => a.keyframes), [
    [{ transform: 'translateY(0%)' }, { transform: 'translateY(100%)' }],
    [{ transform: 'translateY(-100%)' }, { transform: 'translateY(0%)' }],
  ]);
  cleanup();
});

test('same-id replacement acquires its own lifetime and detached children release old decoration', () => {
  const x = setup('10'), cleanup = install(x.root);
  x.number.setAttribute('id', 'same-id');
  x.update('11');
  const previous = animations(x.number);
  const replacement = makeNumber('20');
  replacement.number.setAttribute('id', 'same-id');
  replacement.canonical.textContent = 'stale';
  x.number.remove();
  x.root.append(replacement.number);
  emit(x.root, [{ type: 'childList', removedNodes: [x.number], addedNodes: [replacement.number] }]);
  assert.ok(previous.every(animation => animation.cancelled));
  assert.equal(layer(x.number), null);
  assert.equal(replacement.canonical.textContent, '20');
  replacement.number.setAttribute('data-cui-number-value', '21');
  emit(x.root, [{ type: 'attributes', target: replacement.number }]);
  assert.equal(replacement.canonical.textContent, '21');
  assert.equal(animations(replacement.number).length, 2);
  cleanup();
});

test('overlapping installations do not duplicate rolls or dispose another root’s animation', () => {
  const x = setup();
  const disposeInner = install(x.number), disposeOuter = install(x.root);
  x.number.setAttribute('data-cui-number-value', '13');
  for (const root of [x.number, x.root]) emit(root, [{ type: 'attributes', target: x.number }]);
  const running = animations(x.number);
  assert.equal(running.length, 2);
  disposeOuter();
  assert.equal(animations(x.number).length, 2);
  assert.ok(running.every(animation => !animation.cancelled));
  disposeInner();
  assert.ok(running.every(animation => animation.cancelled));
  const cleanup = install(x.root);
  x.update('14');
  assert.equal(animations(x.number).length, 2);
  x.canonical.style.opacity = '.4';
  cleanup();
  assert.equal(x.canonical.style.opacity, '.4', 'later application styles are not overwritten');
});

test('partially failed WAAPI startup cancels created animations without hiding the label', async () => {
  const x = setup(), create = x.doc.createElement;
  let calls = 0, first;
  x.doc.createElement = tag => {
    const el = create(tag);
    el.animate = (...args) => {
      if (++calls === 2) throw new Error('animation unavailable');
      first = FakeElement.prototype.animate.apply(el, args);
      return first;
    };
    return el;
  };
  const cleanup = install(x.root);
  x.update('13');
  await flush();
  assert.equal(first.cancelled, true);
  assert.equal(layer(x.number), null);
  assert.equal(x.canonical.style.opacity, '');
  assert.equal(x.canonical.textContent, '13');
  cleanup();
});

test('installer is idempotent, cleans up safely without browser APIs, and can be reinstalled', () => {
  const doc = { listeners: new Set(), addEventListener(_name, fn) { this.listeners.add(fn); }, removeEventListener(_name, fn) { this.listeners.delete(fn); } };
  const root = { ownerDocument: doc, querySelectorAll: () => [] };
  const cleanup = install(root);
  assert.equal(install(root), cleanup);
  cleanup(); cleanup();
  assert.equal(doc.listeners.size, 0);
  const second = install(root);
  assert.notEqual(second, cleanup);
  second();
});

test('static fallback remains canonical; clipping is declared and browser code owns no transport', async () => {
  const directory = new URL('../../packages/vanilla/components/number/', import.meta.url);
  const fixture = JSON.parse(await readFile(new URL('fixtures/static.json', directory), 'utf8'));
  assert.match(fixture.expectedHtml, /data-cui-number-motion="none"/);
  assert.match(fixture.expectedHtml, /class="cui-number__canonical">\$1,204\.50/);
  assert.doesNotMatch(fixture.expectedHtml, /aria-live|__roll/);
  const css = await readFile(new URL('styles.css', directory), 'utf8');
  assert.match(css, /\.cui-number__column\s*\{[^}]*overflow:\s*hidden/);
  assert.match(css, /prefers-reduced-motion/);
  const source = await readFile(new URL('interaction.js', directory), 'utf8');
  assert.doesNotMatch(source, /fetch\s*\(|XMLHttpRequest|WebSocket|data-cui-operation|dispatchEvent/);
});
