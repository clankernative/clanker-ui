import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
// Import the browser module as ESM without adding a package manifest or dependencies.
const source = await readFile(new URL("../../../clanker-ui-gallery/ui/navigation.js", import.meta.url), "utf8");
const navigation = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);

test("normalizes route slashes without changing the root", () => {
  assert.equal(navigation.normalizePathname(""), "/");
  assert.equal(navigation.normalizePathname("/tasks/42///"), "/tasks/42");
  assert.equal(navigation.normalizePathname("components/"), "/components");
});

test("matches workspace routes including task detail", () => {
  assert.deepEqual(navigation.navigationTarget("/"), { section: "overview", page: "overview" });
  assert.deepEqual(navigation.navigationTarget("/tasks/42/"), { section: "tasks", page: "task-detail" });
  assert.deepEqual(navigation.navigationTarget("/tasks"), { section: "tasks", page: "tasks" });
  assert.deepEqual(navigation.navigationTarget("/layouts/"), { section: "layouts", page: "layouts" });
});

test("uses the selected component and the server default when name is absent", () => {
  assert.deepEqual(navigation.navigationTarget("/components", "?name=metric&q=stats"), {
    section: "components", page: "component", name: "metric",
  });
  assert.deepEqual(navigation.navigationTarget("/components/", "?q=button"), {
    section: "components", page: "component", name: "button",
  });
});

test("normalizes persisted category and scroll state", () => {
  assert.deepEqual(navigation.normalizeNavigationState({
    groups: { layout: true, forms: false, invalid: "open" }, scrollTop: 144,
  }), { groups: { layout: true, forms: false }, scrollTop: 144 });
  assert.deepEqual(navigation.normalizeNavigationState({ groups: [], scrollTop: -1 }), {
    groups: {}, scrollTop: null,
  });
  assert.deepEqual(navigation.normalizeNavigationState(null), { groups: {}, scrollTop: null });
});
