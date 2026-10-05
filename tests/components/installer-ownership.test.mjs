import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

async function load(path) {
  const source = await readFile(new URL(path, import.meta.url), 'utf8');
  return import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
}
const [{ install: installModal }, { install: installDrawer }, { install: installCommandMenu }] = await Promise.all([
  load('../../packages/vanilla/components/modal/interaction.js'),
  load('../../packages/vanilla/components/drawer/interaction.js'),
  load('../../packages/vanilla/components/command-menu/interaction.js'),
]);

class Root {
  constructor(parent = null) { this.parent = parent; this.listeners = new Map(); this.nodes = new Set(); this.dialogs = []; }
  addEventListener(type, callback) { const list = this.listeners.get(type) || []; list.push(callback); this.listeners.set(type, list); }
  removeEventListener(type, callback) { this.listeners.set(type, (this.listeners.get(type) || []).filter((x) => x !== callback)); }
  contains(node) { return this.nodes.has(node) || node === this; }
  querySelectorAll(selector) { return selector.includes('command-menu') ? this.nodesFor?.(selector) || [] : this.dialogs; }
  emit(type, event) { for (const fn of this.listeners.get(type) || []) fn(event); }
}
class Node {
  constructor(selector) { this.selector = selector; this.isConnected = true; this.focusCount = 0; this.attributes = new Map(); }
  closest(selector) { return this.selector === selector ? this : this.closestNode?.closest?.(selector) || null; }
  matches(selector) { return this.selector === selector || (selector === '[data-cui-modal-dialog]' && this.selector === '[data-cui-modal-dialog]'); }
  focus() { this.focusCount++; }
  getAttribute(key) { return this.attributes.get(key) ?? null; }
  setAttribute(key, value) { this.attributes.set(key, String(value)); }
  removeAttribute(key) { this.attributes.delete(key); }
}
function overlay(install, root, componentSelector) {
  const component = new Node(componentSelector), trigger = new Node('[data-cui-modal-trigger]'), dialog = new Node('[data-cui-modal-dialog]');
  trigger.closestNode = component; dialog.closestNode = component; dialog.open = false;
  dialog.showModal = function () { this.open = true; };
  dialog.close = function () { this.open = false; root.emit('close', { target: this }); root.parent?.emit('close', { target: this }); };
  component.querySelector = (selector) => selector === '[data-cui-modal-dialog]' ? dialog : selector === '[data-cui-modal-trigger]' ? trigger : null;
  root.nodes.add(component); root.nodes.add(trigger); root.dialogs.push(dialog);
  return { component, trigger, dialog, dispose: install(root) };
}
function click(target) { let prevented = false; return { target, preventDefault() { prevented = true; }, get defaultPrevented() { return prevented; } }; }

test('modal and drawer teardown closes only dialogs leased by that exact overlapping installation', () => {
  for (const [install, selector] of [[installModal, '[data-cui-component="modal"]'], [installDrawer, '[data-cui-component="drawer"]']]) {
    const outer = new Root(), inner = new Root(outer); const owned = overlay(install, inner, selector);
    const disposeOuter = install(outer);
    const event = click(owned.trigger); inner.emit('click', event); outer.emit('click', event);
    assert.equal(event.defaultPrevented, true); assert.equal(owned.dialog.open, true);
    disposeOuter(); assert.equal(owned.dialog.open, true); assert.equal(owned.trigger.focusCount, 0);
    owned.dispose(); assert.equal(owned.dialog.open, false); assert.equal(owned.trigger.focusCount, 0);
  }
});

test('modal and drawer ignore externally opened dialogs and detached openers never regain focus', () => {
  for (const [install, selector] of [[installModal, '[data-cui-component="modal"]'], [installDrawer, '[data-cui-component="drawer"]']]) {
    const root = new Root(), x = overlay(install, root, selector);
    x.dialog.open = true; x.dialog.close(); assert.equal(x.trigger.focusCount, 0);
    const event = click(x.trigger); x.dialog.open = false; root.emit('click', event); root.nodes.delete(x.trigger); x.trigger.isConnected = false;
    x.dialog.close(); assert.equal(x.trigger.focusCount, 0); x.dispose();
  }
});

function commandFixture(root) {
  const component = new Node('[data-cui-component="command-menu"]'), trigger = new Node('[data-cui-command-trigger]');
  const dialog = new Node('[data-cui-command-dialog]'), search = new Node('[data-cui-command-search]');
  const item = new Node('[data-cui-command-item]'), group = new Node('[data-cui-command-group]'), empty = new Node('[data-cui-command-empty]');
  for (const node of [trigger, dialog, search, item, group, empty]) node.closestNode = component;
  search.value = 'existing query'; item.hidden = false; item.id = 'option-alpha'; item.href = '/alpha'; group.hidden = false; empty.hidden = true;
  search.focus = () => { search.focusCount++; root.activeElement = search; };
  item.getAttribute = (key) => key === 'data-cui-command-text' ? 'alpha' : item.attributes.get(key) ?? null;
  item.toggleAttribute = (key, enabled) => enabled ? item.setAttribute(key, '') : item.removeAttribute(key);
  item.click = () => {};
  component.querySelector = (selector) => selector === '[data-cui-command-dialog]' ? dialog : selector === '[data-cui-command-trigger]' ? trigger : selector === '[data-cui-command-search]' ? search : selector === '[data-cui-command-empty]' ? empty : selector.includes('data-cui-command-item') ? item : null;
  component.querySelectorAll = (selector) => selector.includes('data-cui-command-item') ? (selector.includes(':not([hidden])') && item.hidden ? [] : [item]) : selector.includes('data-cui-command-group') ? [group] : [];
  group.querySelector = () => item.hidden ? null : item;
  dialog.open = false; dialog.showModal = function () { this.open = true; };
  dialog.close = function () { this.open = false; root.emit('close', { target: this }); root.parent?.emit('close', { target: this }); };
  root.nodesFor = () => [component]; root.nodes.add(component); root.nodes.add(trigger); root.nodes.add(search); root.nodes.add(item); root.nodes.add(dialog);
  return { component, trigger, dialog, search, item, group, empty };
}
test('command menu overlapping cleanup preserves another owner and later app writes', () => {
  const outer = new Root(), inner = new Root(outer), fixture = commandFixture(inner);
  const disposeInner = installCommandMenu(inner), disposeOuter = installCommandMenu(outer);
  const event = click(fixture.trigger); inner.emit('click', event); outer.emit('click', event);
  assert.equal(fixture.dialog.open, true); assert.equal(fixture.search.value, '');
  disposeOuter(); assert.equal(fixture.dialog.open, true); assert.equal(fixture.search.value, '');
  fixture.search.value = 'app-owned later value'; fixture.item.hidden = true; fixture.item.setAttribute('aria-selected', 'app-value');
  disposeInner();
  assert.equal(fixture.dialog.open, false);
  assert.equal(fixture.search.value, 'app-owned later value'); assert.equal(fixture.item.hidden, true);
  assert.equal(fixture.item.getAttribute('aria-selected'), 'app-value');
});

test('command menu combobox keeps DOM focus and exposes its active linked option to assistive technology', () => {
  const root = new Root(), fixture = commandFixture(root), dispose = installCommandMenu(root);
  assert.equal(root.listeners.get('click')?.length, 1); assert.equal(root.contains(fixture.trigger), true);
  assert.equal(fixture.trigger.closest('[data-cui-command-trigger]'), fixture.trigger);
  assert.equal(fixture.trigger.closest('[data-cui-component="command-menu"]'), fixture.component);
  assert.equal(fixture.component.querySelector('[data-cui-command-dialog]'), fixture.dialog);
  const opened = click(fixture.trigger); root.emit('click', opened);
  assert.equal(opened.defaultPrevented, true); assert.equal(fixture.dialog.open, true);
  assert.equal(fixture.search.focusCount, 1); assert.equal(fixture.item.focusCount, 0);
  assert.equal(fixture.search.getAttribute('aria-activedescendant'), fixture.item.id);
  assert.equal(fixture.item.getAttribute('aria-selected'), 'true');
  const down = { target: fixture.search, key: 'ArrowDown', preventDefault() { this.defaultPrevented = true; } };
  root.emit('keydown', down);
  assert.equal(down.defaultPrevented, true);
  assert.equal(fixture.search.focusCount, 1); assert.equal(fixture.item.focusCount, 0);
  assert.equal(fixture.search.getAttribute('aria-activedescendant'), fixture.item.id);
  let navigated = 0; fixture.item.click = () => { navigated++; };
  const enter = { target: fixture.search, key: 'Enter', preventDefault() { this.defaultPrevented = true; } };
  root.emit('keydown', enter);
  assert.equal(navigated, 1); assert.equal(fixture.item.href, '/alpha');
  dispose();
});

test('command menu query reset is limited to the installation containing its trigger', () => {
  const outer = new Root(), inner = new Root(outer), fixture = commandFixture(inner);
  const dispose = installCommandMenu(outer); fixture.search.value = 'user draft';
  const event = click(fixture.trigger); outer.emit('click', event);
  assert.equal(event.defaultPrevented, false); assert.equal(fixture.search.value, 'user draft'); assert.equal(fixture.dialog.open, false);
  dispose();
});
