import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const root = new URL('../../packages/vanilla/', import.meta.url);
const names = ['modal', 'drawer', 'popover', 'command-menu', 'confirm-dialog', 'date-calendar', 'date-picker', 'file-upload', 'data-viewport'];

test('complete components advertise unavailable Native seams independently', () => {
  for (const name of names) {
    const component = JSON.parse(readFileSync(new URL(`components/${name}/component.json`, root)));
    assert.equal(component.status, 'ready', name);
    assert.equal(component.integration.native.status, 'adapter-required', name);
    assert.ok(component.integration.native.scope.length > 20, name);
    assert.ok(component.assets.scripts.length, `${name}: real browser behavior is declared`);
  }
});

test('app-owned ports resolve to declared component-owned TypeScript contracts', () => {
  let portCount = 0;
  for (const name of names) {
    const component = JSON.parse(readFileSync(new URL(`components/${name}/component.json`, root)));
    for (const port of component.integration.ports ?? []) {
      portCount++;
      assert.ok(['input', 'output'].includes(port.direction), name);
      assert.ok(component.assets.contracts.includes(port.typeSource), name);
      assert.ok(port.typeSource.startsWith(`components/${name}/`), name);
      const source = readFileSync(new URL(port.typeSource, root), 'utf8');
      assert.match(source, new RegExp(`export\\s+(?:interface|type|class)\\s+${port.export}\\b`), name);
      assert.doesNotThrow(() => stripTypeScriptTypes(source), `${name}: TypeScript declaration syntax`);
      assert.ok(port.purpose.length > 20, name);
    }
  }
  assert.ok(portCount >= 5, 'confirmation, both dates, file selection, and viewport ports are discoverable');
});
