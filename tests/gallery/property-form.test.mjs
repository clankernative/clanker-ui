import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

const source = await readFile(new URL("../../../clanker-ui-gallery/ui/property-form.js", import.meta.url), "utf8");
const propertyForm = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
const { defaultValues, validateValues, mountPropertyForm } = propertyForm;

const descriptor = {
  name: "sample",
  fields: [
    { name: "label", attribute: "label", label: "Label", kind: "text", required: true, default: null, choices: [], help: "" },
    { name: "tone", attribute: "tone", label: "Tone", kind: "enum", required: false, default: "neutral", choices: ["neutral", "danger"], help: "" },
    { name: "enabled", attribute: "enabled", label: "Enabled", kind: "boolean", required: false, default: false, choices: [], help: "" },
    { name: "count", attribute: "count", label: "Count", kind: "number", required: false, default: null, choices: [], minimum: 0, maximum: 10, help: "" },
    { name: "detail", attribute: "detail", label: "Detail", kind: "multiline", required: false, default: null, choices: [], help: "", visibleWhen: { field: "enabled", equals: true } },
    { name: "settings", attribute: "settings", label: "Settings", kind: "object", required: false, default: null, choices: [], help: "", fields: [
      { name: "mode", attribute: "mode", label: "Mode", kind: "enum", required: true, default: null, choices: ["compact", "wide"], help: "" },
    ] },
    { name: "items", attribute: "items", label: "Items", kind: "list", required: false, default: [], choices: [], help: "", fields: [
      { name: "value", attribute: "value", label: "Value", kind: "text", required: true, default: null, choices: [], help: "" },
      { name: "active", attribute: "active", label: "Active", kind: "boolean", required: true, default: false, choices: [], help: "" },
    ] },
  ],
  slots: [],
};

test("every ready contract has an exported Rust sample that validates with the shared form", async () => {
  const catalog = JSON.parse(await readFile(new URL("../../packages/vanilla/property-catalog.json", import.meta.url), "utf8"));
  const directory = new URL("../../packages/vanilla/components/", import.meta.url);
  const ready = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    let manifest;
    try { manifest = JSON.parse(await readFile(new URL(`${entry.name}/component.json`, directory), "utf8")); }
    catch (error) { if (error.code === "ENOENT") continue; throw error; }
    if (manifest.status === "ready") ready.push(manifest.name);
  }
  assert.deepEqual(catalog.components.map(component => component.name).sort(), ready.sort());
  for (const component of catalog.components) {
    const withSlots = { ...component, slots: component.slots.map(slot => ({ ...slot, samples: ["sample"] })) };
    const result = validateValues(withSlots, component.sample);
    assert.equal(result.valid, true, `${component.name}: ${JSON.stringify(result.errors)}`);
  }
});

test("native itemKind string lists are editable and remain scalar", () => {
  const native = { fields: [{ name: "columns", kind: "list", itemKind: "text", required: true, default: [], fields: [] }], slots: [] };
  const result = validateValues(native, { columns: ["Name", "State"] });
  assert.equal(result.valid, true);
  assert.deepEqual(result.values.columns, ["Name", "State"]);
});

test("defaults preserve typed defaults and initialize generic enum, boolean, and collection fields", () => {
  assert.deepEqual(defaultValues(descriptor), {
    label: "", tone: "neutral", enabled: false, count: null, detail: null,
    settings: null, items: [],
  });
  const first = defaultValues(descriptor);
  first.items.push({ value: "changed" });
  assert.deepEqual(defaultValues(descriptor).items, [], "defaults are fresh, not shared");
});

test("validates enum and boolean values and omits hidden conditional fields", () => {
  const result = validateValues(descriptor, {
    label: "Okay", tone: "danger", enabled: false, detail: "hidden text",
    settings: { mode: "wide" }, items: [],
  });
  assert.equal(result.valid, true);
  assert.deepEqual(result.values, {
    label: "Okay", tone: "danger", enabled: false, count: null,
    settings: { mode: "wide" }, items: [],
  });
});

test("requires visible conditional values and accepts them when visible", () => {
  const conditional = { ...descriptor, fields: descriptor.fields.map((field) => field.name === "detail" ? { ...field, required: true, visibleWhen: undefined, visible_when: { field: "enabled", equals: true } } : field) };
  const hidden = validateValues(conditional, { label: "Okay", enabled: false });
  assert.equal(hidden.valid, true);
  assert.equal(Object.hasOwn(hidden.values, "detail"), false);
  const shown = validateValues(conditional, { label: "Okay", enabled: true, detail: "Shown" });
  assert.equal(shown.valid, true);
  assert.equal(shown.values.detail, "Shown");
  assert.ok(validateValues(conditional, { label: "Okay", enabled: true }).errors.some((error) => error.path === "detail"));
});

test("rejects invalid finite numbers, range violations, and blank required text", () => {
  for (const count of [NaN, Infinity, -1, 11]) {
    const result = validateValues(descriptor, { label: "Okay", count });
    assert.equal(result.valid, false, `count ${count} must fail`);
    assert.ok(result.errors.some((error) => error.path === "count"));
  }
  for (const label of ["", "   ", 42]) {
    const result = validateValues(descriptor, { label });
    assert.ok(result.errors.some((error) => error.path === "label"));
  }
});

test("supports null optional values and validates nested object and list fields", () => {
  const valid = validateValues(descriptor, { label: "Okay", tone: null, count: null, settings: { mode: "compact" }, items: [{ value: "one", active: true }] });
  assert.equal(valid.valid, true);
  assert.deepEqual(valid.values.items, [{ value: "one", active: true }]);
  assert.equal(valid.values.tone, null);
  const invalid = validateValues(descriptor, { label: "Okay", settings: { mode: "unknown" }, items: [{ value: "   ", active: true }] });
  assert.ok(invalid.errors.some((error) => error.path === "settings.mode"));
  assert.ok(invalid.errors.some((error) => error.path === "items[0].value"));
});

test("rejects unknown root and nested fields instead of forwarding them", () => {
  const result = validateValues(descriptor, { label: "Okay", unexpected: "value", settings: { mode: "compact", injected: true } });
  assert.equal(result.valid, false);
  assert.ok(result.errors.some((error) => error.path === "unexpected"));
  assert.ok(result.errors.some((error) => error.path === "settings.injected"));
});

test("handles scalar string lists and keeps record lists nested", () => {
  const lists = { name: "lists", fields: [
    { name: "columns", kind: "list", required: true, default: [], fields: [{ name: "item", kind: "text", required: true }] },
    { name: "rows", kind: "list", required: false, default: [], fields: [
      { name: "cells", kind: "list", required: true, default: [], fields: [{ name: "value", kind: "text", required: true }] },
    ] },
  ], slots: [] };
  const valid = validateValues(lists, { columns: ["Name", "Status"], rows: [{ cells: ["A", "Ready"] }] });
  assert.equal(valid.valid, true);
  assert.deepEqual(valid.values, { columns: ["Name", "Status"], rows: [{ cells: ["A", "Ready"] }] });
  assert.ok(validateValues(lists, { columns: [""] }).errors.some((error) => error.path === "columns[0]"));
});

test("required slots default to their first admitted sample; optional slots default to null", () => {
  const withSlots = { fields: [], slots: [
    { name: "body", required: true, samples: [{ value: "sample-card", label: "Example card" }] },
    { name: "actions", required: false, samples: ["sample-actions"] },
  ] };
  assert.deepEqual(defaultValues(withSlots), { slots: { body: "sample-card", actions: null } });
  assert.deepEqual(validateValues(withSlots, {}).values, { slots: { body: "sample-card" } });
  assert.ok(validateValues(withSlots, { slots: { body: "<b>raw</b>" } }).errors.some((error) => error.path === "slots.body"));
});

class TestElement {
  constructor(tagName, ownerDocument) {
    this.tagName = tagName;
    this.ownerDocument = ownerDocument;
    this.children = [];
    this.attributes = new Map();
    this.listeners = new Map();
    this.value = "";
    this.checked = false;
    this.disabled = false;
  }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  removeAttribute(name) { this.attributes.delete(name); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  addEventListener(name, callback) { const callbacks = this.listeners.get(name) ?? []; callbacks.push(callback); this.listeners.set(name, callbacks); }
  dispatch(name) { for (const callback of this.listeners.get(name) ?? []) callback({ target: this, preventDefault() {} }); }
  click() { this.dispatch("click"); }
  get textContent() { return this._text ?? this.children.map((child) => child.textContent ?? "").join(""); }
  set textContent(value) { this._text = String(value); this.children = []; }
}

function testContainer() {
  const document = { createElement: (name) => new TestElement(name, document), createTextNode: (text) => ({ textContent: String(text) }) };
  return new TestElement("container", document);
}
function allElements(root) { return [root, ...root.children.flatMap((child) => child.children ? allElements(child) : [child])]; }

test("optional toggles are limited to nullable fields and empty help rows are omitted", () => {
  const container = testContainer();
  mountPropertyForm(container, { name: "toggles", fields: [
    { name: "variant", kind: "enum", label: "Variant", required: false, default: "standard", choices: ["standard", "compact"], help: "" },
    { name: "enabled", kind: "boolean", label: "Enabled", required: false, default: false, help: "" },
    { name: "note", kind: "text", label: "Note", required: false, default: null, help: "" },
  ], slots: [] });
  const elements = allElements(container);
  const toggles = elements.filter((element) => element.getAttribute?.("data-property-role") === "enable");
  assert.equal(toggles.length, 1);
  assert.equal(toggles[0].getAttribute("data-property"), "note");
  assert.equal(elements.some((element) => element.className === "cui-property-field-help"), false);
});

test("mount uses QA data-property selectors and preserves draft when external validation returns a string", () => {
  const container = testContainer();
  const validConfigs = [];
  const mounted = mountPropertyForm(container, { name: "small", fields: [
    { name: "label", kind: "text", label: "Label", required: true, default: null },
  ], slots: [] }, {
    initial: { label: "ready" },
    validate: (values) => values.label === "blocked" ? "Rejected by preview validation" : undefined,
    onValid: (values) => validConfigs.push(values),
  });
  const input = allElements(container).find((element) => element.getAttribute("data-property") === "label");
  assert.ok(input, "QA can select [data-property=\"label\"]");
  assert.equal(input.name, "label");
  assert.ok(input.className.includes("property-control"));
  input.value = "blocked";
  input.dispatch("input");
  assert.equal(mounted.getDraft().label, "blocked", "validation failure retains the draft");
  assert.equal(validConfigs.length, 1, "onValid is not called for an externally rejected config");
  assert.ok(allElements(container).some((element) => element.textContent?.includes("Rejected by preview validation")));
  input.value = "accepted";
  input.dispatch("input");
  assert.equal(validConfigs.at(-1).label, "accepted");
  mounted.reset();
  assert.equal(mounted.getDraft().label, "ready", "reset restores the admitted representative sample");
});

test("mounted native scalar lists and nested records are attached to the form", () => {
  const container = testContainer();
  mountPropertyForm(container, {fields:[
    {name:"columns",kind:"list",itemKind:"text",required:true,default:[]},
    {name:"rows",kind:"list",required:false,default:[],fields:[{name:"cells",kind:"list",itemKind:"text",required:true,default:[]}]}
  ],slots:[]},{initial:{columns:["Name"],rows:[{cells:["Sample"]}]}});
  const inputs = allElements(container).filter(element => element.tagName === "input");
  assert.ok(inputs.some(input => input.getAttribute("data-property") === "columns[0]"));
  assert.ok(inputs.some(input => input.getAttribute("data-property") === "rows[0].cells[0]"));
});

test("nullable numeric controls enable and integer fields reject fractions", () => {
  const container = testContainer();
  const descriptor = {fields:[{name:"count",kind:"number",integer:true,required:false,default:null,minimum:0,maximum:10}],slots:[]};
  const mounted = mountPropertyForm(container,descriptor);
  let elements=allElements(container);
  assert.equal(elements.find(element => element.name === "count").disabled,true);
  const toggle=elements.find(element => element.name === "count.__enabled");
  toggle.checked=true;toggle.dispatch("change");
  assert.equal(mounted.getDraft().count,1);
  assert.equal(validateValues(descriptor,{count:1.5}).valid,false);
});

test("literal TextValue contracts use one text control without exposing binding variants", () => {
  const container = testContainer();
  const descriptor = {fields:[{name:"label",kind:"object",required:true,default:null,fields:[
    {name:"kind",kind:"enum",required:true,default:"literal",choices:["literal"]},
    {name:"value",kind:"text",required:true,default:null}
  ]}],slots:[]};
  const mounted = mountPropertyForm(container,descriptor,{initial:{label:{kind:"literal",value:"Continue"}}});
  const controls=allElements(container).filter(element => element.tagName === "input");
  assert.equal(controls.length,1);
  assert.equal(controls[0].name,"label.value");
  controls[0].value="Updated";controls[0].dispatch("input");
  assert.deepEqual(mounted.getDraft().label,{kind:"literal",value:"Updated"});
});
