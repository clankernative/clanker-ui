import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const entry = await readFile(new URL('../../../clanker-ui-gallery/ui/app.js', import.meta.url), 'utf8');
const explorer = await readFile(new URL('../../../clanker-ui-gallery/ui/explorer.js', import.meta.url), 'utf8');

test('first-paint bootstrap only eagerly loads navigation, not the Explorer graph', () => {
  const staticImports = entry.match(/^import .+ from .+;$/gm);
  assert.deepEqual(staticImports, ["import { initializeNavigation } from './navigation.js';"]);
  assert.ok(entry.indexOf('initializeNavigation();') < entry.indexOf("import('./explorer.js')"));
  assert.match(entry, /if \(document\.querySelector\('\[data-property-workbench\]'\)\)/);
  assert.match(entry, /if \(document\.querySelector\('\[data-task-navigation\]'\)\)/);
  assert.match(entry, /import\('\.\/query-navigation\.js'\)/);
  assert.doesNotMatch(entry, /datastar-fetch/);
});

test('lazy Explorer retains locked metadata and the shared property form', () => {
  assert.match(explorer, /import catalog from '\.\/clanker-properties\.js'/);
  assert.match(explorer, /export function initializeExplorer/);
  assert.match(explorer, /mountPropertyForm\(container, descriptor/);
  assert.doesNotMatch(explorer, /initializeNavigation|DOMContentLoaded|datastar-fetch/);
});
