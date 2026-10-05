import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const galleryRoot = path.resolve(packageRoot, '../clanker-ui-gallery');
const nativeNames = [
  'activity-feed', 'alert', 'avatar', 'badge', 'breadcrumbs', 'button', 'button-group', 'card', 'checkbox-group', 'cluster', 'container', 'copy-field', 'cover', 'data-table', 'definition-list', 'disclosure', 'divider', 'empty-state', 'filter-bar', 'form-field', 'grid', 'icon', 'layer', 'metric', 'page-header', 'pane', 'pagination', 'progress', 'progress-steps', 'radio-group', 'reel', 'segmented-control', 'select-field', 'sidebar', 'skeleton', 'split', 'stack', 'status-indicator', 'switch', 'tabs', 'tag', 'theme-switcher', 'toast', 'toggle', 'tooltip',
].sort();
const adapterRequired = ['command-menu', 'confirm-dialog', 'data-viewport', 'date-calendar', 'date-picker', 'drawer', 'file-upload', 'modal', 'popover'].sort();
const namesFrom = source => [...source.matchAll(/is_([a-z][a-z0-9_]*)\s*:\s*doc\.name\s*==\s*"([a-z0-9-]+)"/g)].map(([, , name]) => name).sort();
const recordFieldCount = (source, name) => {
  const record = source.match(new RegExp(`^\\t${name} : \\{\\n([\\s\\S]*?)^\\t\\}`, 'm'));
  assert.ok(record, `${name} record declaration`);
  return [...record[1].matchAll(/^\t\t[A-Za-z_][A-Za-z0-9_]*\s*:/gm)].length;
};

test('gallery Native inventory and specimen branches cover exactly the 45 admitted components', async () => {
  const inventory = await readFile(path.join(galleryRoot, 'shared/ComponentDocs.roc'), 'utf8');
  const query = await readFile(path.join(galleryRoot, 'queries/components/Components.roc'), 'utf8');
  const template = await readFile(path.join(galleryRoot, 'ui/pages/components.html'), 'utf8');
  const docs = [...inventory.matchAll(/doc\("([a-z0-9-]+)"/g)].map(([, name]) => name).sort();
  assert.deepEqual(docs, nativeNames);
  assert.deepEqual(namesFrom(query), nativeNames);
  assert.match(query, /specimen\s*:\s*SpecimenSelection/, 'ComponentsView exposes the nested specimen record');
  assert.match(query, /specimen: \{ description: "Selected component's closed Native specimen branch flags\.", fields:/, 'output contract describes the nested specimen record');
  assert.equal(recordFieldCount(query, 'SpecimenSelection'), nativeNames.length, 'all 45 selection flags remain present in the nested record');
  assert.ok(recordFieldCount(query, 'SpecimenSelection') <= 64, 'SpecimenSelection stays within the host field budget');
  assert.ok(recordFieldCount(query, 'ComponentsView') <= 64, 'ComponentsView stays within the host field budget');
  for (const name of nativeNames) {
    const flag = name.replaceAll('-', '_');
    assert.match(query, new RegExp(`is_${flag} : Bool`), `${name} nested view flag`);
    assert.match(template, new RegExp(`components\\.specimen\\.is_${flag}\\s*%\\}`), `${name} nested template branch`);
  }
  const fixtureCases = inventory.slice(inventory.indexOf('fixture_names :'), inventory.indexOf('presets_from_names :'));
  const fixtureNames = {
    'activity-feed': 'typical', 'button-group': 'typical', 'checkbox-group': 'typical', 'copy-field': 'typical',
    'definition-list': 'typical', disclosure: 'contained-golden', 'progress-steps': 'typical-golden',
    'radio-group': 'typical', 'segmented-control': 'typical', tabs: 'typical', 'theme-switcher': 'text-golden',
    toast: 'inline', toggle: 'typical', tooltip: 'top-golden',
  };
  for (const [name, fixture] of Object.entries(fixtureNames)) {
    assert.match(fixtureCases, new RegExp(`"${name}"\\s*=>\\s*\\[\\"${fixture}"\\]`), `${name} named representative fixture`);
    const manifest = JSON.parse(await readFile(path.join(packageRoot, 'packages/vanilla/components', name, 'component.json'), 'utf8'));
    assert.equal(manifest.status, 'ready', `${name} remains component-ready`);
    assert.notEqual(manifest.integration?.native?.status, 'adapter-required', `${name} is Native-admitted`);
    const fixtureFile = fixture.endsWith('-golden') ? `${fixture}.json` : `${fixture}-golden.json`;
    assert.ok(manifest.fixtures.includes(`components/${name}/fixtures/${fixtureFile}`), `${name} selects an existing package fixture`);
  }
  assert.match(query, /all\.len\(\) == 45/);
  assert.match(template, /45 Native-admitted component contracts/);
});

test('the nine adapter-required contracts stay out of Native branches and are called out separately', async () => {
  const template = await readFile(path.join(galleryRoot, 'ui/pages/components.html'), 'utf8');
  const query = await readFile(path.join(galleryRoot, 'queries/components/Components.roc'), 'utf8');
  for (const name of adapterRequired) {
    assert.doesNotMatch(query, new RegExp(`doc\\("${name}"`), `${name} is not in the Native inventory`);
    assert.doesNotMatch(template, new RegExp(`<cui-${name}(?:\\s|>)`), `${name} has no Native declaration`);
  }
  assert.match(template, /browser fixture\/simulator-adapter demonstrations are available via the gallery-owned launcher/);
  assert.match(template, /Drawer, Confirm Dialog, Date Calendar, Popover, Date Picker, File Upload, Command Menu, Data Viewport, and Modal/);
});
