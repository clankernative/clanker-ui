import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { install } from '../../packages/vanilla/components/number/interaction.js';

// Deterministic DOM substitute; CSS transition rendering is qualified separately
// in the browser harness and is not simulated by these tests.
class FakeElement {
  constructor(tag = 'span') {
    this.tagName = tag;
    this.attrs = new Map();
    this.children = [];
    this.parent = null;
    this.nodeType = 1;
    this.listeners = new Map();
    this._className = '';
    this.style = {
      opacity: '',
      custom: new Map(),
      setProperty(key, value) { this.custom.set(key, String(value)); },
      getPropertyValue(key) { return this.custom.get(key) ?? ''; },
      removeProperty(key) { this.custom.delete(key); this[key] = ''; },
    };
  }
  get className() { return this._className; }
  set className(value) { this._className = String(value); }
  get parentNode() { return this.parent; }
  get nextSibling() {
    if (!this.parent) return null;
    const siblings = this.parent.children;
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }
  matches(selector) {
    if (selector.startsWith('.')) return this.className.split(/\s+/).includes(selector.slice(1));
    return selector === '[data-cui-component="number"]' && this.getAttribute('data-cui-component') === 'number';
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  querySelectorAll(selector) {
    return this.children.flatMap(child => [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)]);
  }
  append(...children) { for (const child of children) this.insertBefore(child, null); }
  insertBefore(node, reference) {
    if (node === reference) return node;
    if (node.parent === this && node.nextSibling === reference) return node;
    node.remove();
    const index = reference == null ? this.children.length : this.children.indexOf(reference);
    if (index < 0) throw new Error('reference node is not a child');
    node.parent = this;
    this.children.splice(index, 0, node);
    return node;
  }
  replaceChildren(...children) {
    for (const child of [...this.children]) child.remove();
    this.append(...children);
  }
  replaceWith(node) {
    if (!this.parent) return;
    const parent = this.parent;
    const index = parent.children.indexOf(this);
    node.remove();
    this.remove();
    parent.insertBefore(node, parent.children[index] ?? null);
  }
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
  get textContent() { return this._text ?? this.children.map(child => child.textContent).join(''); }
  set textContent(value) {
    for (const child of [...this.children]) child.remove();
    this._text = String(value);
  }
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
function makeNumber(value, { motion = 'roll', duration = '320', direction = 'up' } = {}) {
  const number = new FakeElement();
  for (const [key, val] of Object.entries({
    'data-cui-component': 'number',
    'data-cui-number-motion': motion,
    'data-cui-number-duration': duration,
    'data-cui-number-direction': direction,
    'data-cui-number-value': value,
  })) number.setAttribute(key, val);
  const canonical = new FakeElement();
  canonical.className = 'cui-number__canonical';
  canonical.textContent = value;
  number.append(canonical);
  return { number, canonical };
}
function setup(value = '12', reducedMotion = false, options = {}) {
  const media = {
    matches: reducedMotion, listeners: new Set(),
    addEventListener(_name, fn) { this.listeners.add(fn); },
    removeEventListener(_name, fn) { this.listeners.delete(fn); },
  };
  const view = {
    Element: FakeElement,
    MutationObserver: FakeObserver,
    matchMedia: () => media,
    getComputedStyle: () => ({ direction: 'ltr' }),
  };
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
  const { number, canonical } = makeNumber(value, options);
  number.ownerDocument = doc;
  root.append(number);
  const update = label => {
    number.setAttribute('data-cui-number-value', label);
    emit(root, [{ type: 'attributes', target: number }]);
  };
  return { root, number, canonical, doc, media, view, update };
}
const odometer = number => number.querySelector('.cui-number__odometer');
const columns = number => number.querySelectorAll('.cui-number__digit');
function shownLabel(number) {
  const direction = number.getAttribute('data-cui-number-direction');
  return (odometer(number)?.children ?? []).map(node => {
    if (!node.matches('.cui-number__digit')) return node.textContent;
    const index = Number(node.style.getPropertyValue('--cui-number-digit'));
    return String(direction === 'down' ? 9 - index : index);
  }).join('');
}


test('first paint stays static; roll roots acquire one aria-hidden odometer seeded without animation', () => {
  const x = setup('$1,204.50');
  assert.equal(odometer(x.number), null, 'server markup starts with only canonical text');
  const cleanup = install(x.root);
  assert.equal(x.canonical.textContent, '$1,204.50');
  assert.equal(odometer(x.number).getAttribute('aria-hidden'), 'true');
  assert.equal(shownLabel(x.number), '$1,204.50');
  assert.equal(odometer(x.number).style.getPropertyValue('--cui-number-duration'), '320ms');
  assert.equal(columns(x.number).some(node => node.matches('.cui-number__digit--entering')), false);
  assert.equal(x.canonical.getAttribute('aria-live'), null);
  assert.equal(odometer(x.number).getAttribute('aria-live'), null);
  cleanup();
  assert.equal(odometer(x.number), null);
});

test('roll odometer renders one column per digit and down direction reverses strip orientation', () => {
  const up = setup('$4,201.8K'), cleanUp = install(up.root);
  assert.equal(columns(up.number).length, 5);
  assert.equal(shownLabel(up.number), '$4,201.8K');
  const upStrip = columns(up.number)[0].querySelector('.cui-number__strip');
  assert.equal(upStrip.children.map(node => node.textContent).join(''), '0123456789');
  cleanUp();

  const down = setup('42', false, { direction: 'down' }), cleanDown = install(down.root);
  assert.equal(columns(down.number)[0].querySelector('.cui-number__strip').children.map(node => node.textContent).join(''), '9876543210');
  assert.equal(shownLabel(down.number), '42');
  cleanDown();
});

test('compatible updates retarget custom properties on the same column nodes', () => {
  const x = setup('12'), cleanup = install(x.root);
  const [tens, ones] = columns(x.number);
  assert.equal(tens.style.getPropertyValue('--cui-number-digit'), '1');
  assert.equal(ones.style.getPropertyValue('--cui-number-digit'), '2');
  x.update('13');
  assert.deepEqual(columns(x.number), [tens, ones]);
  assert.equal(ones.style.getPropertyValue('--cui-number-digit'), '3');
  assert.equal(x.canonical.textContent, '13');
  assert.equal(shownLabel(x.number), '13');
  cleanup();
});

test('an update during a CSS transition retargets in place without replacing columns', () => {
  const x = setup('100,200'), cleanup = install(x.root);
  const original = [...columns(x.number)];
  x.update('100,201');
  const afterFirst = [...columns(x.number)];
  x.update('100,209');
  assert.deepEqual(afterFirst, original);
  assert.deepEqual(columns(x.number), original, 'the interrupted roll retains every column node');
  assert.equal(x.canonical.textContent, '100,209');
  assert.equal(shownLabel(x.number), '100,209');
  cleanup();
});

test('digit growth and shrink align from the right and keep the ones-place column', () => {
  const x = setup('99,999'), cleanup = install(x.root);
  const ones = columns(x.number).at(-1);
  x.update('100,000');
  assert.equal(columns(x.number).length, 6);
  assert.equal(columns(x.number).at(-1), ones);
  assert.equal(columns(x.number)[0].matches('.cui-number__digit--entering'), true);
  assert.equal(shownLabel(x.number), '100,000');

  const grouped = setup('999'), cleanGrouped = install(grouped.root);
  const groupedOnes = columns(grouped.number).at(-1);
  grouped.update('1,000');
  assert.equal(columns(grouped.number).at(-1), groupedOnes);
  assert.equal(shownLabel(grouped.number), '1,000');
  grouped.update('999');
  assert.equal(columns(grouped.number).length, 3, 'removed leading columns do not remain stale');
  assert.equal(columns(grouped.number).at(-1), groupedOnes);
  assert.equal(shownLabel(grouped.number), '999');
  cleanup();
  cleanGrouped();
});

test('incompatible prefixes, suffixes, separators and glyphs snap without extra label layers', () => {
  for (const [before, after] of [
    ['-12', '+13'], ['$12', '€13'], ['4.2K', '4.3M'], ['1.2', '1,3'],
    ['12', '١٣'], ['12', '7'.repeat(65)],
  ]) {
    const x = setup(before), cleanup = install(x.root);
    const previous = odometer(x.number);
    x.update(after);
    assert.equal(x.canonical.textContent, after);
    assert.notEqual(odometer(x.number), previous, `${before} -> ${after} rebuilds on snap`);
    assert.equal(x.number.children.length, 2, 'canonical and one odometer are the only layers');
    assert.equal(shownLabel(x.number), after);
    assert.equal(odometer(x.number).getAttribute('aria-hidden'), 'true');
    assert.doesNotMatch(odometer(x.number).className, /roll|whole/);
    cleanup();
  }
});

test('motion none returns to canonical; reduced motion, hidden pages and zero duration snap', () => {
  const staticNumber = setup('12'), cleanStatic = install(staticNumber.root);
  staticNumber.number.setAttribute('data-cui-number-motion', 'none');
  staticNumber.update('13');
  assert.equal(staticNumber.canonical.textContent, '13');
  assert.equal(odometer(staticNumber.number), null);
  cleanStatic();

  for (const configure of [
    x => { x.media.matches = true; },
    x => { x.doc.hidden = true; },
    x => x.number.setAttribute('data-cui-number-duration', '0'),
    x => x.number.setAttribute('data-cui-number-duration', '1001'),
    x => { x.view.getComputedStyle = () => ({ direction: 'rtl' }); },
  ]) {
    const x = setup('12'), cleanup = install(x.root);
    const previous = columns(x.number)[0];
    configure(x);
    x.update('13');
    assert.equal(x.canonical.textContent, '13');
    assert.equal(shownLabel(x.number), '13');
    assert.equal(columns(x.number)[0] === previous, false, 'ineligible updates rebuild immediately');
    assert.equal(columns(x.number).some(node => node.matches('.cui-number__digit--entering')), false);
    if (x.number.getAttribute('data-cui-number-duration') === '0') {
      assert.equal(odometer(x.number).style.getPropertyValue('--cui-number-duration'), '0ms');
    }
    cleanup();
  }
});

test('canonical text synchronously tracks every accepted value and reduced-motion changes snap current output', () => {
  const x = setup('12'), cleanup = install(x.root);
  x.update('13');
  assert.equal(x.canonical.textContent, '13');
  x.update('14');
  assert.equal(x.canonical.textContent, '14');
  x.media.matches = true;
  for (const listener of x.media.listeners) listener();
  assert.equal(x.canonical.textContent, '14');
  assert.equal(shownLabel(x.number), '14');
  cleanup();
});

test('stable-id before-patch retains the owned node and updates its canonical value synchronously', () => {
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
  const oldColumns = [...columns(original)];
  currentRegion.dispatchEvent({ type: 'cui:before-patch', bubbles: true, detail: { current: currentRegion, incoming: incomingRegion } });
  assert.equal(incomingRegion.children[0], original);
  assert.equal(original.getAttribute('data-cui-number-value'), '13');
  assert.equal(x.canonical.textContent, '13', 'canonical is current before the host replaceChildren call');
  assert.deepEqual(columns(original), oldColumns);
  assert.equal(shownLabel(original), '13');
  currentRegion.replaceChildren(...incomingRegion.children);
  emit(x.root, [{ type: 'attributes', target: original }]);
  assert.equal(original.getAttribute('data-cui-operation'), null);
  assert.equal(original.getAttribute('data-cui-number-evil'), null);
  assert.equal(x.root.listeners.has('cui:before-patch'), true);
  cleanup();
  assert.equal(x.root.listeners.size, 0);
});

test('removing an owned number or exact installer root removes its odometer and releases listeners', () => {
  for (const installOnNumber of [false, true]) {
    const x = setup('12');
    const root = installOnNumber ? x.number : x.root;
    const cleanup = install(root);
    assert.ok(odometer(x.number));
    root.remove();
    emit(x.doc, [{ type: 'childList', removedNodes: [root] }]);
    assert.equal(odometer(x.number), null);
    assert.equal(x.media.listeners.size, 0);
    assert.equal(x.doc.listeners.size, 0);
    assert.ok(FakeObserver.instances.filter(observer => observer.target === root).every(observer => observer.disconnected));
    cleanup();
  }

  const x = setup('12'), cleanup = install(x.root);
  x.number.remove();
  emit(x.root, [{ type: 'childList', removedNodes: [x.number] }]);
  assert.equal(odometer(x.number), null, 'detached owned children release their decoration');
  cleanup();
});

test('a replacement number with the same id receives a fresh lease after the old node is removed', () => {
  const x = setup('10'), cleanup = install(x.root);
  x.number.id = 'same-id';
  x.update('11');
  const previous = x.number;
  const replacement = makeNumber('20');
  replacement.number.id = 'same-id';
  previous.remove();
  x.root.append(replacement.number);
  emit(x.root, [{ type: 'childList', removedNodes: [previous], addedNodes: [replacement.number] }]);
  assert.equal(odometer(previous), null);
  assert.equal(replacement.canonical.textContent, '20');
  replacement.number.setAttribute('data-cui-number-value', '21');
  emit(x.root, [{ type: 'attributes', target: replacement.number }]);
  assert.equal(replacement.canonical.textContent, '21');
  assert.equal(shownLabel(replacement.number), '21');
  cleanup();
});

test('install is idempotent and overlapping installers preserve lease ownership', () => {
  const x = setup();
  const inner = install(x.number);
  assert.equal(install(x.number), inner);
  const outer = install(x.root);
  const original = [...columns(x.number)];
  x.number.setAttribute('data-cui-number-value', '13');
  emit(x.number, [{ type: 'attributes', target: x.number }]);
  emit(x.root, [{ type: 'attributes', target: x.number }]);
  assert.deepEqual(columns(x.number), original);
  outer();
  assert.ok(odometer(x.number), 'the non-owning outer cleanup does not remove the inner lease');
  inner();
  assert.equal(odometer(x.number), null);
});

test('static fallback and stylesheet keep canonical visible without JavaScript and use CSS only', async () => {
  const directory = new URL('../../packages/vanilla/components/number/', import.meta.url);
  const fixture = JSON.parse(await readFile(new URL('fixtures/static.json', directory), 'utf8'));
  assert.match(fixture.expectedHtml, /data-cui-number-motion="none"/);
  assert.match(fixture.expectedHtml, /class="cui-number__canonical">\$1,204\.50/);
  assert.doesNotMatch(fixture.expectedHtml, /aria-live|__odometer/);
  const css = await readFile(new URL('styles.css', directory), 'utf8');
  assert.match(css, /:has\(> \.cui-number__odometer\)[\s\S]*?opacity:\s*0/);
  assert.match(css, /transition:\s*transform var\(--cui-number-duration, 320ms\) cubic-bezier\(\.2,\.8,\.2,1\)/);
  assert.match(css, /@starting-style/);
  assert.match(css, /prefers-reduced-motion/);
  const source = await readFile(new URL('interaction.js', directory), 'utf8');
  assert.doesNotMatch(source, /\.animate\s*\(|Animation|transitionWholeLabel|cui-number__roll/);
  assert.doesNotMatch(source, /fetch\s*\(|XMLHttpRequest|WebSocket|data-cui-operation|dispatchEvent/);
});
