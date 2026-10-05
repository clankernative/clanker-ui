import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

async function load(path) {
  const source = await readFile(new URL(path, import.meta.url), 'utf8');
  return import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
}

const [{ install: installToasts }, { install: installTooltips }, { install: installThemeSwitcher }] = await Promise.all([
  load('../../packages/vanilla/components/toast/install.js'),
  load('../../packages/vanilla/components/tooltip/install.js'),
  load('../../packages/vanilla/components/theme-switcher/install.js'),
]);

function root() {
  const target = new EventTarget();
  target.querySelectorAll = () => [];
  return target;
}

test('all installers accept an empty root and return idempotent cleanup functions', () => {
  const previousDocument = globalThis.document;
  const previousWindow = globalThis.window;
  const document = root();
  document.hidden = false;
  document.activeElement = null;
  const window = root();
  globalThis.document = document;
  globalThis.window = window;
  try {
    for (const install of [installToasts, installTooltips, installThemeSwitcher]) {
      const cleanup = install(root());
      assert.equal(typeof cleanup, 'function');
      assert.doesNotThrow(cleanup);
      assert.doesNotThrow(cleanup);
    }
  } finally {
    if (previousDocument === undefined) delete globalThis.document; else globalThis.document = previousDocument;
    if (previousWindow === undefined) delete globalThis.window; else globalThis.window = previousWindow;
  }
});

class Target extends EventTarget {}

function toastFixture(timeout) {
  let removed = false;
  const toast = {
    dataset: {},
    style: { setProperty() {} },
    hasAttribute: (name) => name === 'data-cui-toast-timeout',
    getAttribute: (name) => name === 'data-cui-toast-timeout' ? String(timeout) : null,
    matches: () => false,
    contains: () => false,
    remove: () => { removed = true; },
  };
  const region = { childNodes: [], querySelectorAll: () => [toast] };
  const root = new Target();
  root.querySelectorAll = () => [region];
  return { root, region, toast, get removed() { return removed; } };
}

test('toast timer expires, while teardown cancels pending timer mutation', async () => {
  const previousDocument = globalThis.document;
  const previousWindow = globalThis.window;
  globalThis.document = Object.assign(new Target(), { hidden: false, activeElement: null });
  globalThis.window = new Target();
  try {
    const expiring = toastFixture(1000);
    const stopExpired = installToasts(expiring.root);
    await new Promise((resolve) => setTimeout(resolve, 1050));
    assert.equal(expiring.removed, true);
    stopExpired();

    const pending = toastFixture(1000);
    const stopPending = installToasts(pending.root);
    stopPending();
    await new Promise((resolve) => setTimeout(resolve, 1050));
    assert.equal(pending.removed, false);
  } finally {
    if (previousDocument === undefined) delete globalThis.document; else globalThis.document = previousDocument;
    if (previousWindow === undefined) delete globalThis.window; else globalThis.window = previousWindow;
  }
});

test('removing a toast through DOM mutation immediately releases its pending timer', async () => {
  const previousDocument = globalThis.document;
  const previousWindow = globalThis.window;
  const previousObserver = globalThis.MutationObserver;
  let observerCallback;
  globalThis.MutationObserver = class {
    constructor(callback) { observerCallback = callback; }
    observe() {}
    disconnect() {}
  };
  globalThis.document = Object.assign(new Target(), { hidden: false, activeElement: null });
  globalThis.window = new Target();
  try {
    const fixture = toastFixture(1000);
    const stop = installToasts(fixture.root);
    const region = fixture.region;
    observerCallback([{ removedNodes: [region.querySelectorAll()[0]], addedNodes: [] }]);
    await new Promise((resolve) => setTimeout(resolve, 1050));
    assert.equal(fixture.removed, false);
    stop();
  } finally {
    if (previousDocument === undefined) delete globalThis.document; else globalThis.document = previousDocument;
    if (previousWindow === undefined) delete globalThis.window; else globalThis.window = previousWindow;
    if (previousObserver === undefined) delete globalThis.MutationObserver; else globalThis.MutationObserver = previousObserver;
  }
});

test('a toast region added after installation receives timer ownership', async () => {
  const previousDocument = globalThis.document;
  const previousWindow = globalThis.window;
  const previousObserver = globalThis.MutationObserver;
  let observerCallback;
  globalThis.MutationObserver = class {
    constructor(callback) { observerCallback = callback; }
    observe() {}
    disconnect() {}
  };
  globalThis.document = Object.assign(new Target(), { hidden: false, activeElement: null });
  globalThis.window = new Target();
  try {
    const fixture = toastFixture(1000);
    fixture.root.querySelectorAll = () => [];
    fixture.region.matches = (selector) => selector === '[data-cui-component="toast"]';
    fixture.region.querySelectorAll = (selector) => selector === '[data-cui-toast]' ? [fixture.toast] : [];
    const stop = installToasts(fixture.root);
    observerCallback([{ removedNodes: [], addedNodes: [fixture.region] }]);
    await new Promise((resolve) => setTimeout(resolve, 1050));
    assert.equal(fixture.removed, true);
    stop();
  } finally {
    if (previousDocument === undefined) delete globalThis.document; else globalThis.document = previousDocument;
    if (previousWindow === undefined) delete globalThis.window; else globalThis.window = previousWindow;
    if (previousObserver === undefined) delete globalThis.MutationObserver; else globalThis.MutationObserver = previousObserver;
  }
});
