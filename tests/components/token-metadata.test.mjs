import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
const root = new URL('../../packages/vanilla/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('ui-package.json', root), 'utf8'));

test('token metadata is locked, component-owned, and preserves shared ownership', () => {
  assert.ok(manifest.resources.includes(manifest.tokenMetadata));
  const theme = JSON.parse(readFileSync(new URL(manifest.tokenMetadata, root), 'utf8'));
  const owners = new Map(theme.tokens.map(token => [token.name, 'theme']));
  assert.equal(owners.size, theme.tokens.length);
  const components = readdirSync(new URL('components/', root), { withFileTypes: true }).filter(entry => entry.isDirectory()).map(entry => entry.name).sort().map(directory => [directory, JSON.parse(readFileSync(new URL(`components/${directory}/component.json`, root), 'utf8'))]);
  for (const [directory, component] of components) {
    for (const token of component.tokenDescriptions || []) {
      assert.ok(component.tokens.includes(token.name), `${directory}: undeclared ${token.name}`);
      assert.ok(!owners.has(token.name), `${token.name}: duplicate owner`);
      assert.ok(token.purpose.trim().length > 0);
      assert.ok(!('default' in token) && !('defaultValue' in token), 'CSS remains default authority');
      owners.set(token.name, directory);
    }
  }
  for (const [directory, component] of components) {
    for (const token of component.tokens) assert.ok(owners.has(token), `${directory}: missing ${token} metadata`);
  }
  const heading = theme.tokens.find(token => token.name === '--cui-heading-text');
  assert.equal(heading.role, 'text'); assert.equal(heading.semantic, 'heading');
  assert.equal(owners.get('--cui-text'), 'theme');
});

test('shared heading fallbacks remain local instead of freezing aliases at root', () => {
  const defaults = readFileSync(new URL(manifest.theme, root), 'utf8');
  for (const [component, names] of [
    ['page-header', ['--cui-page-header-title-text']],
    ['card', ['--cui-card-title-text']],
    ['empty-state', ['--cui-empty-state-title-text']],
    ['data-table', ['--cui-data-table-caption-text', '--cui-data-table-head-text']],
  ]) {
    const css = readFileSync(new URL(`components/${component}/styles.css`, root), 'utf8');
    for (const name of names) {
      assert.match(defaults, new RegExp(`${name}:\\s*initial\\s*;`));
      assert.match(css, new RegExp(`var\\(${name},\\s*var\\(--cui-heading-text`));
    }
  }
});
