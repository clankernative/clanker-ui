import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

async function load(path) {
  await readFile(new URL(path, import.meta.url), 'utf8');
  return import(new URL(path, import.meta.url));
}
const [{ install: installCalendar, parseCalendarDate }, { install: installPicker }] = await Promise.all([
  load('../../packages/vanilla/components/date-calendar/interaction.js'),
  load('../../packages/vanilla/components/date-picker/interaction.js'),
]);

class Observer {
  static instances = new Set();
  constructor(callback) { this.callback = callback; this.connected = false; Observer.instances.add(this); }
  observe() { this.connected = true; }
  disconnect() { this.connected = false; }
  static flush() { for (const observer of this.instances) if (observer.connected) observer.callback([]); }
}
class Classes {
  constructor(node) { this.node = node; this.values = new Set(); }
  add(value) { this.values.add(value); this.node.attributes.set('class', [...this.values].join(' ')); }
  contains(value) { return this.values.has(value); }
}
function selectorParts(selector) { return selector.split(',').map((part) => part.trim()).filter(Boolean); }
function matchesSimple(node, selector) {
  if (!(node instanceof Element)) return false;
  const tag = selector.match(/^[a-zA-Z][\w-]*/)?.[0];
  if (tag && node.tagName.toLowerCase() !== tag.toLowerCase()) return false;
  for (const match of selector.matchAll(/\[([^\]]+)\]/g)) {
    const [name, expected] = match[1].split('=').map((x) => x?.replace(/^['"]|['"]$/g, ''));
    if (!node.hasAttribute(name) || (expected !== undefined && node.getAttribute(name) !== expected)) return false;
  }
  return true;
}
class EventHub {
  constructor() { this.listeners = new Map(); }
  addEventListener(type, callback, options = {}) {
    if (options.signal?.aborted) return;
    const list = this.listeners.get(type) || []; list.push({ callback, options }); this.listeners.set(type, list);
    options.signal?.addEventListener('abort', () => this.removeEventListener(type, callback), { once: true });
  }
  removeEventListener(type, callback) { this.listeners.set(type, (this.listeners.get(type) || []).filter((item) => item.callback !== callback)); }
  dispatchEvent(event) {
    if (!event.target) Object.defineProperty(event, 'target', { configurable: true, value: this });
    Object.defineProperty(event, 'currentTarget', { configurable: true, value: this });
    for (const { callback } of [...(this.listeners.get(event.type) || [])]) callback.call(this, event);
    if (event.bubbles && !event.cancelBubble && this.parentElement) this.parentElement.dispatchEvent(event);
    else if (event.bubbles && !event.cancelBubble && this.ownerDocument) this.ownerDocument.dispatchEvent(event);
    return !event.defaultPrevented;
  }
}
class Element extends EventHub {
  constructor(tag, doc) {
    super(); this.tagName = tag.toUpperCase(); this.ownerDocument = doc; this.parentElement = null; this.children = []; this.attributes = new Map(); this.dataset = new Proxy({}, { set: (target, key, value) => { target[key] = String(value); this.attributes.set(`data-${String(key).replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`, String(value)); return true; }, deleteProperty: (target, key) => { delete target[key]; this.attributes.delete(`data-${String(key).replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`); return true; } }); this.style = { values: {}, priorities: {}, setProperty(k, v, priority = '') { this.values[k] = String(v); this.priorities[k] = priority; }, getPropertyValue(k) { return this.values[k] || ''; }, getPropertyPriority(k) { return this.priorities[k] || ''; }, removeProperty(k) { const previous = this.values[k] || ''; delete this.values[k]; delete this.priorities[k]; return previous; } }; this.classList = new Classes(this); this.textContent = ''; this.disabled = false; this.tabIndex = -1; this.value = ''; this.open = false;
  }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  get tabIndex() { return Number(this.getAttribute('tabindex') ?? -1); }
  set tabIndex(value) { this.setAttribute('tabindex', String(value)); }
  hasAttribute(name) { return this.attributes.has(name); }
  removeAttribute(name) { this.attributes.delete(name); }
  append(child) { child.parentElement = this; this.children.push(child); }
  replaceChildren(...children) { this.children = []; for (const child of children.flatMap((x) => x instanceof Fragment ? x.children : [x])) this.append(child); }
  contains(node) { return node === this || this.children.some((child) => child.contains(node)); }
  matches(selector) { return selectorParts(selector).some((part) => matchesSimple(this, part)); }
  closest(selector) { for (let current = this; current; current = current.parentElement) if (current.matches(selector)) return current; return null; }
  querySelectorAll(selector) { const found = []; const walk = (node) => { for (const child of node.children) { if (selectorParts(selector).some((part) => matchesSimple(child, part))) found.push(child); walk(child); } }; walk(this); return found; }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  focus() { this.ownerDocument.activeElement = this; }
  getBoundingClientRect() { return this.rect || { left: 20, right: 120, top: 20, bottom: 50, width: 100, height: 30 }; }
  get form() { return this._form || null; }
  set form(value) { this._form = value; }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter((x) => x !== this); this.parentElement = null; }
}
class Fragment { constructor() { this.children = []; } append(child) { this.children.push(child); } }
class Document extends EventHub {
  constructor() {
    super(); this.nodeType = 9; this.defaultView = Object.assign(new EventHub(), { AbortController, Event, CustomEvent, MutationObserver: Observer, innerWidth: 320, innerHeight: 240, setTimeout, clearTimeout, getComputedStyle: () => ({ direction: 'ltr' }) });
    this.documentElement = new Element('html', this); this.activeElement = null;
  }
  createElement(tag) { return new Element(tag, this); }
  createDocumentFragment() { return new Fragment(); }
  querySelectorAll(selector) { return this.documentElement.querySelectorAll(selector); }
  querySelector(selector) { return this.documentElement.querySelector(selector); }
  matches() { return false; }
  contains(node) { return this.documentElement.contains(node); }
}
function el(doc, parent, tag, attrs = {}) {
  const node = doc.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === 'dataset') Object.assign(node.dataset, value);
    else if (key === 'value') node.value = value;
    else if (key === 'disabled') node.disabled = value;
    else node.setAttribute(key, value);
  }
  parent?.append(node); return node;
}
const months = ['Jan|uary', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
const weekdays = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
function calendar(doc, parent, { month = '2024-02-01', first = 0, selection = '', end = '', today = '2024-02-14', min = '', max = '', unavailable = [], labels = months } = {}) {
  const root = el(doc, parent, 'section', { 'data-cui-date-calendar': '', 'data-cui-date-interactive': 'true', dataset: { cuiDateMonth: month, cuiDateFirstDay: String(first), cuiDateMonthLabels: JSON.stringify(labels), cuiDateWeekdayShort: JSON.stringify(weekdays.map((x) => x.slice(0, 2))), cuiDateWeekdayFull: JSON.stringify(weekdays), cuiDateSelectionStart: selection, cuiDateSelectionEnd: end, cuiDateSelectionMode: 'range', cuiDateToday: today, cuiDateMinimum: min, cuiDateMaximum: max } });
  el(doc, root, 'button', { 'data-cui-date-previous': '' }); el(doc, root, 'h2', { 'data-cui-date-month-label': '' }); el(doc, root, 'button', { 'data-cui-date-next': '' });
  const body = el(doc, root, 'tbody', { 'data-cui-date-grid-body': '' });
  for (const value of unavailable) el(doc, root, 'data', { 'data-cui-date-unavailable': '', value });
  return { root, body, get days() { return body.querySelectorAll('[data-cui-date-day]'); }, get prev() { return root.querySelector('[data-cui-date-previous]'); }, get next() { return root.querySelector('[data-cui-date-next]'); } };
}
function click(node) { node.dispatchEvent(new Event('click', { bubbles: true })); }
function key(node, name, options = {}) { const event = new Event('keydown', { cancelable: true, bubbles: true }); Object.defineProperties(event, { key: { value: name }, shiftKey: { value: !!options.shiftKey } }); node.dispatchEvent(event); return event; }
function byDate(cal, value) { return cal.days.find((button) => button.dataset.cuiDateValue === value); }
function mount(doc, node) { doc.documentElement.append(node); return node; }

function pickerFixture(doc, parent, { range = true, dropdown = false, start = '2024-02-10', end = '', month = '2024-02-01' } = {}) {
  const owner = el(doc, parent, 'section', { 'data-cui-date-picker': '', dataset: { cuiDateMode: range ? 'range' : 'single' } });
  const form = el(doc, owner, 'form');
  const startInput = el(doc, form, 'input', { type: 'date', 'data-cui-date-start': '', value: start }); startInput.form = form;
  const endInput = range ? el(doc, form, 'input', { type: 'date', 'data-cui-date-end': '', value: end }) : null; if (endInput) endInput.form = form;
  let dropdownNode, summary, panel;
  if (dropdown) { dropdownNode = el(doc, owner, 'details', { 'data-cui-date-dropdown': '' }); summary = el(doc, dropdownNode, 'summary'); panel = el(doc, dropdownNode, 'div', { 'data-cui-date-panel': '' }); }
  const cal = calendar(doc, dropdown ? panel : owner, { month, selection: start, end });
  return { owner, form, startInput, endInput, dropdown: dropdownNode, summary, panel, cal };
}

 test('month buttons change the visible month and are bounded by month interval, not first day', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '2024-02-01', min: '2024-01-31', max: '2024-03-01' });
  const stop = installCalendar(cal.root);
  assert.equal(cal.prev.disabled, false); assert.equal(cal.next.disabled, false);
  click(cal.prev); assert.equal(cal.root.dataset.cuiDateMonth, '2024-01-01');
  assert.equal(cal.root.querySelector('[data-cui-date-month-label]').textContent, 'Jan|uary 2024');
  click(cal.prev); assert.equal(cal.root.dataset.cuiDateMonth, '2024-01-01');
  click(cal.next); assert.equal(cal.root.dataset.cuiDateMonth, '2024-02-01'); stop();
});

test('JSON labels preserve pipes; malformed arrays do not enhance the server-rendered calendar', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')));
  const stop = installCalendar(cal.root);
  assert.ok(cal.days.some((button) => button.getAttribute('aria-label')?.includes('Jan|uary')));
  assert.equal(cal.days.length, 42); stop();
  cal.root.dataset.cuiDateMonthLabels = 'Jan|Feb|Mar'; Observer.flush(); const stopInvalid = installCalendar(cal.root);
  assert.equal(cal.days.length, 42); assert.equal(cal.root.dataset.cuiDateCalendarReady, undefined); stopInvalid();
});

test('strict ISO parser rejects year 0000, invalid dates, and accepts leap day', () => {
  assert.equal(parseCalendarDate('0000-12-31'), null);
  assert.equal(parseCalendarDate('2024-02-30'), null);
  assert.equal(parseCalendarDate('2024-02-29').getUTCDate(), 29);
});

test('year 0001 and 9999 grid edges render valid dates and padded blank cells', () => {
  for (const [month, expected] of [['0001-01-01', '0001-01-01'], ['9999-12-01', '9999-12-31']]) {
    const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month, today: month });
    const stop = installCalendar(cal.root);
    assert.ok(byDate(cal, expected)); assert.ok(cal.days.length >= 31);
    assert.ok(cal.body.children.some((row) => row.children.some((cell) => cell.hasAttribute('aria-hidden')))); stop();
  }
});

test('no invented 0000 dates and no enabled tab stop when every displayed date is disabled', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '0001-01-01', min: '9999-12-31', max: '9999-12-31' });
  const stop = installCalendar(cal.root);
  assert.ok(cal.days.length > 0); assert.ok(cal.days.every((button) => button.tabIndex === -1));
  assert.ok(cal.days.every((button) => !button.dataset.cuiDateValue.startsWith('0000'))); stop();
});

test('keyboard crossing months, page movement, shift-year movement, clamping and RTL arrows', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '2024-01-01', selection: '2024-01-31' });
  const stop = installCalendar(cal.root), jan31 = byDate(cal, '2024-01-31');
  key(jan31, 'ArrowRight'); assert.equal(cal.root.dataset.cuiDateMonth, '2024-02-01'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-02-01');
  key(doc.activeElement, 'PageUp'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-01-01');
  key(doc.activeElement, 'PageDown', { shiftKey: true }); assert.equal(doc.activeElement.dataset.cuiDateValue, '2025-01-01');
  doc.defaultView.getComputedStyle = () => ({ direction: 'rtl' }); key(doc.activeElement, 'ArrowLeft'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2025-01-02');
  key(doc.activeElement, 'Home'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-12-29');
  key(doc.activeElement, 'End'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2025-01-04'); stop();
});

test('leap-year month movement clamps day and range state exposes selected, range, and today semantics', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '2024-02-01', selection: '2024-01-31', end: '2024-02-29', today: '2024-02-14' });
  const stop = installCalendar(cal.root);
  const startCell = byDate(cal, '2024-01-31').parentElement, endCell = byDate(cal, '2024-02-29').parentElement, todayCell = byDate(cal, '2024-02-14').parentElement;
  assert.equal(startCell.getAttribute('aria-selected'), 'true'); assert.equal(endCell.getAttribute('aria-selected'), 'true');
  assert.equal(todayCell.getAttribute('aria-current'), 'date'); assert.equal(byDate(cal, '2024-02-29').disabled, false);
  key(byDate(cal, '2024-01-31'), 'PageDown'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-02-29'); stop();
});

test('bounded unavailable-date skipping and min/max prevent movement outside constraints', () => {
  const unavailable = Array.from({ length: 4 }, (_, i) => `2024-02-${String(i + 11).padStart(2, '0')}`);
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '2024-02-01', selection: '2024-02-10', min: '2024-02-10', max: '2024-02-15', unavailable });
  const stop = installCalendar(cal.root); key(byDate(cal, '2024-02-10'), 'ArrowRight');
  assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-02-15');
  key(doc.activeElement, 'ArrowRight'); assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-02-15'); stop();
});

test('roving focus falls back inside displayed cells when selection and today are elsewhere', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { month: '2024-02-01', selection: '2030-01-01', today: '2030-01-02' });
  const stop = installCalendar(cal.root); assert.equal(cal.days.filter((button) => button.tabIndex === 0).length, 1); assert.equal(doc.activeElement, null); stop();
});

test('calendar installation is per-root idempotent across runtime and picker owners; cleanup refcounts safely', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')));
  let changes = 0; fixture.cal.root.addEventListener('cui:date-calendar-changed', () => changes++);
  const outer = installCalendar(doc), inner = installCalendar(fixture.cal.root);
  click(byDate(fixture.cal, '2024-02-11')); click(byDate(fixture.cal, '2024-02-11'));
  assert.equal(changes, 2); outer(); click(byDate(fixture.cal, '2024-02-12')); assert.equal(changes, 3);
  inner(); click(byDate(fixture.cal, '2024-02-13')); assert.equal(changes, 3);
});

test('calendar and picker expose bounded typed selection details; native input edits also emit app-facing drafts', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { start: '' });
  const calendarDetails = [], pickerDetails = [];
  fixture.cal.root.addEventListener('cui:date-calendar-changed', (event) => calendarDetails.push(event.detail));
  fixture.owner.addEventListener('cui:date-picker-changed', (event) => pickerDetails.push(event.detail));
  const stop = installPicker(doc);
  click(byDate(fixture.cal, '2024-02-11'));
  assert.deepEqual(calendarDetails.at(-1), { complete: false, end: null, mode: 'range', start: '2024-02-11' });
  assert.deepEqual(pickerDetails.at(-1), { complete: false, end: null, mode: 'range', start: '2024-02-11' });
  fixture.endInput.value = '2024-02-15'; fixture.endInput.dispatchEvent(new Event('input', { bubbles: true }));
  assert.deepEqual(pickerDetails.at(-1), { complete: true, end: '2024-02-15', mode: 'range', start: '2024-02-11' });
  stop();
});

test('picker rejects malformed, wrong-mode, reversed, and constraint-violating calendar events', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')));
  fixture.cal.root.dataset.cuiDateMinimum = '2024-02-10';
  const stop = installPicker(doc); let emitted = 0;
  fixture.owner.addEventListener('cui:date-picker-changed', () => emitted++);
  for (const detail of [
    { mode: 'range', start: '2024-02-09', end: null, complete: false },
    { mode: 'single', start: '2024-02-11', end: null, complete: false },
    { mode: 'range', start: '2024-02-15', end: '2024-02-12', complete: true },
    { mode: 'range', start: '2024-02-30', end: null, complete: false },
  ]) fixture.cal.root.dispatchEvent(new CustomEvent('cui:date-calendar-changed', { bubbles: true, detail }));
  assert.equal(emitted, 0); assert.equal(fixture.startInput.value, '2024-02-10'); stop();
});

test('disabled and unavailable dates cannot be selected, including synthetic click events', () => {
  const doc = new Document(), cal = calendar(doc, mount(doc, el(doc, null, 'main')), { unavailable: ['2024-02-11'] });
  const stop = installCalendar(cal.root); let changes = 0;
  cal.root.addEventListener('cui:date-calendar-changed', () => changes++);
  const blocked = byDate(cal, '2024-02-11'); blocked.disabled = false;
  click(blocked);
  assert.equal(changes, 0); assert.equal(cal.root.dataset.cuiDateSelectionStart, ''); stop();
});

test('picker rejects synthetic selections when the matching native date input is disabled', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')));
  const stop = installPicker(doc); let emitted = 0;
  fixture.owner.addEventListener('cui:date-picker-changed', () => emitted++);
  fixture.startInput.disabled = true;
  fixture.cal.root.dispatchEvent(new CustomEvent('cui:date-calendar-changed', { bubbles: true, detail: { mode: 'range', start: '2024-02-11', end: null, complete: false } }));
  assert.equal(fixture.startInput.value, '2024-02-10'); assert.equal(emitted, 0);
  fixture.startInput.disabled = false; fixture.endInput.disabled = true;
  fixture.cal.root.dispatchEvent(new CustomEvent('cui:date-calendar-changed', { bubbles: true, detail: { mode: 'range', start: '2024-02-11', end: '2024-02-12', complete: true } }));
  assert.equal(fixture.startInput.value, '2024-02-10'); assert.equal(fixture.endInput.value, ''); assert.equal(emitted, 0); stop();
});

test('picker releases only its owned panel positioning on replacement and cleanup', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { dropdown: true });
  fixture.panel.style.setProperty('--cui-date-picker-left', 'app-left'); fixture.panel.style.setProperty('max-block-size', 'app-height');
  const stop = installPicker(doc); fixture.dropdown.open = true; fixture.dropdown.dispatchEvent(new Event('toggle'));
  const ownedLeft = fixture.panel.style.getPropertyValue('--cui-date-picker-left');
  fixture.panel.style.setProperty('--cui-date-picker-left', 'later-app-left');
  const oldPanel = fixture.panel; oldPanel.remove();
  const newPanel = el(doc, fixture.dropdown, 'div', { 'data-cui-date-panel': '' }); newPanel.append(fixture.cal.root);
  Observer.flush(); fixture.dropdown.open = true; fixture.dropdown.dispatchEvent(new Event('toggle'));
  assert.equal(oldPanel.style.getPropertyValue('--cui-date-picker-left'), 'later-app-left');
  assert.equal(oldPanel.style.getPropertyValue('max-block-size'), 'app-height');
  newPanel.style.setProperty('--cui-date-picker-left', 'later-new-app-left'); stop();
  assert.equal(newPanel.style.getPropertyValue('--cui-date-picker-left'), 'later-new-app-left');
  assert.notEqual(ownedLeft, '');
});

test('picker positioning cleanup restores original transient panel styles', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { dropdown: true });
  fixture.panel.style.setProperty('--cui-date-picker-left', 'original-left');
  fixture.panel.style.setProperty('--cui-date-picker-top', 'original-top');
  fixture.panel.dataset.cuiDatePanelPlacement = 'app-placement';
  const stop = installPicker(doc); fixture.dropdown.open = true; fixture.dropdown.dispatchEvent(new Event('toggle')); stop();
  assert.equal(fixture.panel.style.getPropertyValue('--cui-date-picker-left'), 'original-left');
  assert.equal(fixture.panel.style.getPropertyValue('--cui-date-picker-top'), 'original-top');
  assert.equal(fixture.panel.dataset.cuiDatePanelPlacement, 'app-placement');
});

test('picker writes both range values before emitting either input/change event and keeps end-month on range completion', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { start: '2024-02-10' });
  const stop = installPicker(doc), observed = [];
  fixture.startInput.addEventListener('input', () => observed.push([fixture.startInput.value, fixture.endInput.value]));
  fixture.endInput.addEventListener('change', () => observed.push([fixture.startInput.value, fixture.endInput.value]));
  click(byDate(fixture.cal, '2024-03-02'));
  assert.deepEqual(observed, [['2024-02-10', '2024-03-02']]);
  assert.equal(fixture.cal.root.dataset.cuiDateMonth, '2024-03-01'); stop();
});

test('reversed range selection orders endpoints and batches both native values before every event', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { start: '2024-02-10' });
  const stop = installPicker(doc), snapshots = [];
  fixture.startInput.addEventListener('input', () => snapshots.push([fixture.startInput.value, fixture.endInput.value]));
  fixture.endInput.addEventListener('input', () => snapshots.push([fixture.startInput.value, fixture.endInput.value]));
  click(byDate(fixture.cal, '2024-02-05'));
  assert.deepEqual(snapshots, [['2024-02-05', '2024-02-10'], ['2024-02-05', '2024-02-10']]); stop();
});

test('picker handlers ignore nested owners and unrelated calendars', () => {
  const doc = new Document(), main = mount(doc, el(doc, null, 'main'));
  const parent = pickerFixture(doc, main), child = pickerFixture(doc, parent.owner);
  let parentEvents = 0, childEvents = 0; parent.owner.addEventListener('cui:date-picker-changed', () => parentEvents++); child.owner.addEventListener('cui:date-picker-changed', () => childEvents++);
  const stop = installPicker(doc); click(byDate(child.cal, '2024-02-11'));
  assert.equal(parent.startInput.value, '2024-02-10'); assert.equal(childEvents, 1); stop();
});

test('picker live morph rebinds replaced calendar, buttons, and inputs without losing native drafts', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main'))), stop = installPicker(doc);
  fixture.startInput.value = '2024-02-12';
  const replacement = calendar(doc, fixture.owner, { month: '2024-02-01' });
  fixture.cal.root.remove(); Observer.flush();
  replacement.root.dataset.cuiDateSelectionStart = fixture.startInput.value; replacement.root.dataset.cuiDateSelectionEnd = '';
  Observer.flush();
  assert.equal(fixture.startInput.value, '2024-02-12');
  click(byDate(replacement, '2024-02-13')); assert.equal(fixture.endInput.value, '2024-02-13');
  const replacementInput = el(doc, fixture.form, 'input', { type: 'date', 'data-cui-date-start': '', value: '2024-02-14' }); replacementInput.form = fixture.form; fixture.startInput.remove(); Observer.flush();
  assert.equal(replacementInput.value, '2024-02-14'); stop();
});

test('owner removal releases handlers and service references; repeated cleanup is safe', () => {
  const doc = new Document(), main = mount(doc, el(doc, null, 'main')), fixture = pickerFixture(doc, main), stop = installPicker(main);
  fixture.owner.remove(); Observer.flush();
  let changes = 0; fixture.cal.root.addEventListener('cui:date-calendar-changed', () => changes++);
  click(byDate(fixture.cal, '2024-02-11')); assert.equal(changes, 0); stop(); stop();
});

test('details dropdown positions within viewport, focuses selected date, and Escape returns focus', () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { dropdown: true }); fixture.summary.rect = { left: 280, right: 310, top: 210, bottom: 235, width: 30, height: 25 }; fixture.panel.getBoundingClientRect = () => ({ width: 180, height: 150 });
  const stop = installPicker(doc); fixture.dropdown.open = true; fixture.dropdown.dispatchEvent(new Event('toggle'));
  assert.equal(doc.activeElement.dataset.cuiDateValue, '2024-02-10'); assert.equal(fixture.panel.dataset.cuiDatePanelPlacement, 'above');
  key(fixture.summary, 'Escape'); assert.equal(fixture.dropdown.open, false); assert.equal(doc.activeElement, fixture.summary);
  fixture.dropdown.open = true; const outside = el(doc, doc.documentElement, 'button'); click(outside); assert.equal(fixture.dropdown.open, false); stop();
});

test('native form reset synchronizes after browser defaults and pending reset work is cancelled on teardown', async () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { start: '2024-02-10' }); fixture.startInput.defaultValue = '2024-02-08';
  const stop = installPicker(doc); fixture.startInput.value = '2024-02-12'; fixture.form.dispatchEvent(new Event('reset'));
  fixture.startInput.value = fixture.startInput.defaultValue; await new Promise((resolve) => setTimeout(resolve, 5));
  assert.equal(fixture.cal.root.dataset.cuiDateSelectionStart, '2024-02-08');
  fixture.form.dispatchEvent(new Event('reset')); stop(); await new Promise((resolve) => setTimeout(resolve, 5)); assert.equal(fixture.cal.root.dataset.cuiDateSelectionStart, '2024-02-08');
});

test('teardown disconnects observers and removes global listeners with no delayed mutations', async () => {
  const doc = new Document(), fixture = pickerFixture(doc, mount(doc, el(doc, null, 'main')), { dropdown: true });
  const previousObservers = new Set(Observer.instances);
  const stopCalendar = installCalendar(doc), stopPicker = installPicker(doc), observerCount = Observer.instances.size;
  fixture.dropdown.open = true; fixture.form.dispatchEvent(new Event('reset')); stopPicker(); stopCalendar();
  const connectedAfter = [...Observer.instances].filter((observer) => !previousObservers.has(observer) && observer.connected).length;
  assert.equal(connectedAfter, 0); assert.ok(observerCount >= 2);
  fixture.dropdown.open = false; await new Promise((resolve) => setTimeout(resolve, 5)); assert.equal(fixture.owner.dataset.cuiDateReady, undefined);
});
