import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('../../../clanker-ui-gallery/ui/preview-bindings.js', import.meta.url), 'utf8');
const { safeDestination, validatePreviewConfiguration, declarationReference } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
const catalog = JSON.parse(await readFile(new URL('../../packages/vanilla/property-catalog.json', import.meta.url), 'utf8'));
const descriptor = name => catalog.components.find(component => component.name === name);

test('destinations match the closed local/HTTP(S) policy without credentials', () => {
  for (const value of ['/', '#details', '?page=2', 'docs/page', 'http://localhost:8080/', 'https://example.test/docs']) assert.equal(safeDestination(value), true, value);
  for (const value of ['', '//example.test', 'javascript:alert(1)', 'data:text/html,x', 'https://user:secret@example.test/', '/bad path', '/bad\\path']) assert.equal(safeDestination(value), false, value);
});

test('nested button and collection destinations cannot bypass validation', () => {
  assert.match(validatePreviewConfiguration('button', {destination:{kind:'link',href:{kind:'literal',value:'javascript:alert(1)'}}}), /safe destination/);
  assert.match(validatePreviewConfiguration('breadcrumbs', {items:[{label:'Unsafe',href:'javascript:alert(1)'},{label:'Current'}]}), /safe local destination/);
  assert.match(validatePreviewConfiguration('pagination', {items:[{kind:'page',label:'2',href:'data:text/html,x'},{kind:'current',label:'1'}]}), /safe local destination/);
});

test('button literal text is unboxed in its Native reference', () => {
  const output = declarationReference(descriptor('button'), {label:{kind:'literal',value:'Continue'},destination:{kind:'action'}});
  assert.match(output, /label="Continue"/);
  assert.doesNotMatch(output, /kind.*literal/);
  assert.match(validatePreviewConfiguration('button',{busy:true,label:{kind:'literal',value:'Continue'}}), /replacement label/);
});

test('Native literal attributes avoid double-escaping and encode JSON separately', () => {
  const button=declarationReference(descriptor('button'),{label:{kind:'literal',value:'A & B'},destination:{kind:'action'}});
  assert.match(button,/label="A & B"/);
  assert.doesNotMatch(button,/&amp;/);
  const table=declarationReference(descriptor('data-table'),{caption:'Sample',columns:["Owner's name"],rows:[]});
  assert.ok(table.includes("columns='[\"Owner\\u0027s name\"]'"));
  assert.throws(()=>declarationReference(descriptor('button'),{label:{kind:'literal',value:'It\'s "ready"'},destination:{kind:'action'}}),/both quote styles/);
});

test('form and select references restore app-owned specimen IDs', () => {
  for (const name of ['form-field','select-field']) assert.match(declarationReference(descriptor(name),descriptor(name).sample), new RegExp(`id="editor-preview-${name}"`));
});

test('pagination represents destination-free previous/next items as disabled', () => {
  const values = {items:[{kind:'previous',label:'Previous'},{kind:'current',label:'1'},{kind:'next',label:'Next'}]};
  assert.equal(validatePreviewConfiguration('pagination',values), null);
  const output = declarationReference(descriptor('pagination'),values);
  assert.equal(output.match(/disabled="true"/g).length,2);
  assert.match(validatePreviewConfiguration('pagination',{items:[{kind:'current',label:'1'},{kind:'page',label:'2'}]}), /destinations for page items/);
});

test('paired actions and Unicode initials reject invalid preview drafts', () => {
  assert.match(validatePreviewConfiguration('empty-state',{actionHref:'/docs'}), /provided together/);
  assert.equal(validatePreviewConfiguration('empty-state',{actionHref:'/docs',actionLabel:'Read docs'}),null);
  assert.equal(validatePreviewConfiguration('avatar',{initials:'👨‍👩‍👦JD'}),null);
  assert.match(validatePreviewConfiguration('avatar',{initials:'FOUR'}), /three Unicode graphemes/);
  assert.match(validatePreviewConfiguration('avatar',{initials:'JD',imageSource:''}), /provenance/);
});
