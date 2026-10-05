import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('../../../clanker-ui-gallery/ui/query-navigation.js', import.meta.url), 'utf8');
const { queryKey, queryValues, safeTaskURL, sameTaskQuery, liveQueryURL, initializerExpression, filterDestination, initializeQueryNavigation } =
  await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

const base = "https://gallery.example/tasks?q=hello&page=2&theme=1&density=1";

test("query values apply the task endpoint defaults without inventing extra state", () => {
  assert.deepEqual(queryValues("/tasks", base), {
    q: "",
    project: "0",
    state: "3",
    sort: "0",
    page: "1",
    theme: "0",
    density: "0",
  });
  assert.deepEqual(queryValues("/tasks?q=alpha&state=2&page=4&theme=1", base), {
    q: "alpha",
    project: "0",
    state: "2",
    sort: "0",
    page: "4",
    theme: "1",
    density: "0",
  });
});

test("query equality ignores parameter order and omitted defaults, but honors every declared field", () => {
  assert.equal(sameTaskQuery("/tasks?q=alpha&page=2", "/tasks?page=2&q=alpha&project=0&state=3&sort=0&theme=0&density=0", base), true);
  assert.equal(sameTaskQuery("/tasks?q=alpha&page=2", "/tasks?q=alpha&page=3", base), false);
  assert.equal(sameTaskQuery("/tasks?q=alpha", "/tasks?q=alpha&theme=1", base), false);
  assert.equal(queryKey("/tasks?unrelated=kept-out", base), null);
  assert.equal(queryKey('/tasks?page=1&page=2', base), null);
});

test("same-page URL validation rejects cross-origin, cross-route, and fragment destinations", () => {
  assert.equal(safeTaskURL("/tasks?q=ok", base)?.pathname, "/tasks");
  assert.equal(safeTaskURL("https://elsewhere.example/tasks", base), null);
  assert.equal(safeTaskURL("/tasks/42", base), null);
  assert.equal(safeTaskURL("/tasks#results", base), null);
  assert.equal(safeTaskURL("//elsewhere.example/tasks", base), null);
  assert.equal(queryValues("javascript:alert(1)", base), null);
  assert.equal(safeTaskURL('https://user:password@gallery.example/tasks', base), null);
  assert.equal(safeTaskURL('/tasks?__proto__=1', base), null);
  assert.equal(safeTaskURL(undefined, base), null);
});

test('subscription keeps document CSP context and encodes query text as a literal', () => {
  const destination = '/tasks?q=' + encodeURIComponent("a\"');alert(1);//") + '&page=2';
  const live = liveQueryURL(destination, '/_live?path=%2Ftasks&image-origins=%5B%22https%3A%2F%2Fimages.example%22%5D', base);
  const parsed = new URL(live, base);
  assert.equal(new URL(parsed.searchParams.get('path'), base).searchParams.get('q'), new URL(destination, base).searchParams.get('q'));
  assert.equal(new URL(parsed.searchParams.get('path'), base).searchParams.get('page'), '2');
  assert.equal(parsed.searchParams.get('image-origins'), '["https://images.example"]');
  assert.ok(initializerExpression(live).startsWith('@get(' + JSON.stringify(live) + ','));
  assert.equal(liveQueryURL('/tasks', 'https://elsewhere.example/_live?image-origins=[]', base), null);
  assert.equal(liveQueryURL('/tasks', '/_live?path=/tasks', base), null);
});

test('submitting new filters from page two always resets paging and retains current preferences', () => {
  class FormDataMock extends Map {
    constructor() { super([['q', 'room'], ['page', '2'], ['theme', '0'], ['density', '0'], ['state', '3']]); }
    append(key, value) { this.set(key, value); }
  }
  const url = filterDestination({ method: 'get', action: '/tasks' }, null, '/tasks?page=2&theme=1&density=1', base, FormDataMock);
  assert.equal(url.searchParams.get('page'), '1');
  assert.equal(url.searchParams.get('theme'), '1');
  assert.equal(url.searchParams.get('density'), '1');
  assert.equal(url.searchParams.get('q'), 'room');
  assert.equal(filterDestination({ method: 'post', action: '/tasks' }, null, '/tasks', base, FormDataMock), null);
});

function lifecycleFixture() {
  const listeners = new Map(), history = [], nodes = new Map();
  const node = (id, extra = {}) => {
    const item = { id, dataset: {}, attributes: new Map(), textContent: '',
      setAttribute(key, value) { this.attributes.set(key, value); },
      getAttribute(key) { return this.attributes.get(key); },
      replaceChildren() { this.textContent = ''; }, querySelector() { return null; },
      focus() { this.focused = true; }, ...extra };
    nodes.set(id, item); return item;
  };
  const region = node('region', { contains: () => true });
  node('task-query', { href: 'https://gallery.example/tasks', dataset: { taskMatching: '9' } });
  node('task-results'); node('task-filter-count'); node('task-navigation-status');
  const makeInitializer = () => ({ id: 'day2-live',
    attributes: new Map([['data-live-url', '/_live?path=%2Ftasks&image-origins=%5B%5D']]),
    getAttribute(key) { return this.attributes.get(key); },
    setAttribute(key, value) { this.attributes.set(key, value); },
    querySelector() { return null; }, cloneNode() { return makeInitializer(); },
    replaceWith(next) { nodes.set('day2-live', next); }
  });
  nodes.set('day2-live', makeInitializer());
  const document = { nodeType: 9, getElementById: id => nodes.get(id),
    querySelector: selector => selector === '[data-task-navigation]' ? region : null,
    addEventListener(type, callback, capture = false) {
      const list = listeners.get(type) ?? []; list.push({ callback, capture }); listeners.set(type, list);
    },
    defaultView: { location: { href: 'https://gallery.example/tasks' },
      history: { state: null, pushState(_state, _title, url) { history.push(['push', url]); },
        replaceState(_state, _title, url) { history.push(['replace', url]); } },
      setTimeout: () => 1, clearTimeout() {}, addEventListener() {} }
  };
  initializeQueryNavigation(document);
  const dispatch = (type, event) => {
    event.stopImmediatePropagation = () => { event.stopped = true; };
    for (const listener of (listeners.get(type) ?? []).sort((a, b) => Number(b.capture) - Number(a.capture))) {
      listener.callback(event); if (event.stopped) break;
    }
    return event;
  };
  const click = href => {
    const link = { href, isConnected: false, hasAttribute: () => false,
      closest: selector => selector === 'a' ? link : {} };
    dispatch('click', { target: link, button: 0, preventDefault() {} });
  };
  return { nodes, history, dispatch, click, region };
}

test('canceled subscriptions cannot deliver patches and history commits only after accepted server metadata', async () => {
  const fixture = lifecycleFixture();
  const old = fixture.nodes.get('day2-live');
  fixture.click('https://gallery.example/tasks?page=2');
  assert.deepEqual(fixture.history, []);
  const stale = fixture.dispatch('datastar-fetch', { detail: { el: old, type: 'datastar-patch-elements' } });
  assert.equal(stale.stopped, true);
  fixture.dispatch('datastar-fetch', { detail: { el: fixture.nodes.get('day2-live'), type: 'datastar-patch-elements' } });
  // The real Datastar watcher morphs these admitted bytes before the microtask.
  fixture.nodes.get('task-query').href = 'https://gallery.example/tasks?page=2';
  await Promise.resolve();
  assert.deepEqual(fixture.history, [['push', '/tasks?page=2']]);
  assert.equal(fixture.region.attributes.get('aria-busy'), 'false');
  assert.equal(fixture.nodes.get('task-results').focused, true);
});

test('same-query refresh finishes even when the metadata attributes do not change', async () => {
  const fixture = lifecycleFixture();
  fixture.click('https://gallery.example/tasks');
  fixture.dispatch('datastar-fetch', { detail: { el: fixture.nodes.get('day2-live'), type: 'datastar-patch-elements' } });
  await Promise.resolve();
  assert.deepEqual(fixture.history, [['replace', '/tasks']]);
  assert.equal(fixture.nodes.get('task-navigation-status').textContent, 'Tasks updated.');
});

test('an initial network interruption retains the accepted URL and restores its stream immediately', () => {
  const fixture = lifecycleFixture();
  fixture.click('https://gallery.example/tasks?page=2');
  fixture.dispatch('datastar-fetch', { detail: { el: fixture.nodes.get('day2-live'), type: 'retrying' } });
  assert.deepEqual(fixture.history, []);
  assert.match(fixture.nodes.get('task-navigation-status').textContent, /previous results are retained/);
  assert.equal(new URL(fixture.nodes.get('day2-live').getAttribute('data-live-url'), base).searchParams.get('path'), '/tasks');
});
