// Execute through ego-browser nodejs against the admitted disposable gallery.
// Prepend globalThis.choiceNativeProof = { origin, space, screenshots, database }.
const fs = await import('node:fs/promises');
const { default: assert } = await import('node:assert/strict');
const { DatabaseSync } = await import('node:sqlite');
const { origin, space, screenshots, database } = globalThis.choiceNativeProof ?? {};
assert.ok(origin && space && screenshots && database, 'provide choiceNativeProof configuration');
const task = await taskSpace(space);
const page = task.page('p1');
const db = new DatabaseSync(database, { readOnly: true });
const statement = db.prepare("select input,status,outcome from day2_invocations where operation='gallery.capture_choices' order by rowid");
const invocations = () => statement.all();
const receipts = [];
function receipt(input, expected, status = 'success') {
  const row = invocations().at(-1);
  assert.deepEqual(JSON.parse(row.input), input);
  assert.equal(row.status, status);
  const outcome = JSON.parse(row.outcome);
  if (status === 'success') assert.deepEqual(outcome.result, expected);
  receipts.push({ input, status, result: outcome.result });
}
async function fresh() {
  // A same-URL goto may be only a fragment navigation and preserve the old draft.
  await page.goto(origin + '/');
  await page.goto(origin + '/layouts#native-choice-proof');
  await page.waitForSelector('#native-choice-form');
}
async function saved(form = '#native-choice-form', noJs = false) {
  await page.focus(`${form} button[type=submit]`);
  await page.keyboard.press('Enter');
  if (noJs) {
    // The ordinary POST redirects to the page; the Saved status is an SSE UI.
    await page.waitForURL(origin + '/layouts');
    await page.waitForSelector('#native-choice-form');
  } else {
    await page.waitForFunction(() => document.querySelector('#day2-command-status')?.textContent.trim() === 'Saved.');
  }
}
try {
  await page.cdp('Emulation.setDeviceMetricsOverride', { width: 1440, height: 2000, deviceScaleFactor: 1, mobile: false });
  await fresh();
  const semantics = await page.evaluate(() => {
    const root = document.querySelector('#native-choice-proof');
    const forms = [...root.querySelectorAll('form')].map(f => ({
      id: f.id, method: f.method, action: new URL(f.action).pathname,
      values: [...new FormData(f)].filter(([name]) => !name.startsWith('_')),
      controls: [...f.querySelectorAll('input:not([type=hidden])')].map(n => ({
        id: n.id, type: n.type, disabled: n.matches(':disabled'), checked: n.checked,
        labelled: n.labels.length > 0, described: n.getAttribute('aria-describedby'),
      })),
    }));
    const ids = [...document.querySelectorAll('[id]')].map(n => n.id);
    return { forms, uniqueIds: new Set(ids).size === ids.length, fieldsets: root.querySelectorAll('fieldset > legend').length,
      unresolved: root.querySelectorAll('cui-toggle,cui-radio-group,cui-checkbox-group').length,
      invalid: root.querySelectorAll('#choice-error-checkboxes[aria-invalid=true], #choice-error-checkboxes [aria-invalid=true]').length,
      overflow: document.documentElement.scrollWidth > innerWidth };
  });
  assert.ok(semantics.uniqueIds);
  assert.equal(semantics.unresolved, 0);
  assert.equal(semantics.overflow, false);
  assert.equal(semantics.fieldsets, 6);
  assert.ok(semantics.invalid > 0);
  assert.ok(semantics.forms.every(f => f.method === 'post' && f.action === '/actions' && f.controls.every(c => c.labelled)));
  assert.deepEqual(semantics.forms[0].values, [['enabled', 'true'], ['density', 'compact'], ['channels', 'email']]);
  assert.deepEqual(semantics.forms[1].values, []);
  assert.ok(semantics.forms[1].controls.every(c => c.disabled));
  assert.equal(semantics.forms[0].controls.filter(c => c.disabled).length, 2);
  await page.screenshot({ path: `${screenshots}/gallery-choice-native-desktop.png` });

  await page.focus('#choice-density-choice-1');
  const sequence = [];
  for (let i = 0; i < 3; i++) {
    await page.keyboard.press('ArrowRight');
    sequence.push(await page.evaluate(() => document.querySelector('#native-choice-form input[name=density]:checked').value));
  }
  assert.deepEqual(sequence, ['comfortable', 'unsupported', 'compact']);
  await page.keyboard.press('ArrowRight');
  await page.focus('#choice-enabled'); await page.press('#choice-enabled', 'Space');
  await page.focus('#choice-channels-choice-1'); await page.press('#choice-channels-choice-1', 'Space');
  await page.focus('#choice-channels-choice-2'); await page.press('#choice-channels-choice-2', 'Space');
  await saved();
  receipt({ channels: ['push'], density: 'comfortable', enabled: false },
    { channel_count: 1, density: 'comfortable', email_selected: false, enabled: false, push_selected: true });

  await fresh();
  await page.click('#choice-enabled'); await page.click('#choice-channels-choice-1');
  await saved();
  receipt({ channels: [], density: 'compact', enabled: false },
    { channel_count: 0, density: 'compact', email_selected: false, enabled: false, push_selected: false });

  await fresh(); await page.click('#choice-channels-choice-2'); await saved();
  receipt({ channels: ['email', 'push'], density: 'compact', enabled: true },
    { channel_count: 2, density: 'compact', email_selected: true, enabled: true, push_selected: true });

  await fresh(); await saved('#native-choice-readonly-form');
  receipt({ channels: ['email'], density: 'comfortable', enabled: true },
    { channel_count: 1, density: 'comfortable', email_selected: true, enabled: true, push_selected: false });

  await fresh();
  await page.click('#choice-enabled'); await page.click('#choice-density-choice-3');
  await page.click('#choice-channels-choice-1'); await page.click('#choice-channels-choice-2');
  await page.focus('#native-choice-form button[type=submit]');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.querySelector('#day2-command-status .error'));
  const rejected = await page.evaluate(() => ({ enabled: document.querySelector('#choice-enabled').checked,
    density: document.querySelector('#native-choice-form input[name=density]:checked').value,
    channels: [...document.querySelectorAll('#native-choice-form input[name=channels]:checked:not(:disabled)')].map(n => n.value),
    disabledChecked: document.querySelector('#choice-channels-choice-3').checked }));
  assert.deepEqual(rejected, { enabled: false, density: 'unsupported', channels: ['push'], disabledChecked: true });
  receipt({ channels: ['push'], density: 'unsupported', enabled: false }, undefined, 'failure');

  await fresh();
  const beforeForgery = invocations().length;
  const forged = await page.evaluate(async () => {
    const form = document.querySelector('#native-choice-form');
    const body = new URLSearchParams(new FormData(form)); body.set('channels', 'disabled');
    const response = await fetch(form.action, { method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' }, body: body.toString() });
    return { status: response.status, text: (await response.text()).slice(0, 1000) };
  });
  assert.equal(forged.status, 400);
  assert.equal(invocations().length, beforeForgery, 'disabled choice must be rejected before invoking the app');

  await page.cdp('Emulation.setDeviceMetricsOverride', { width: 390, height: 1900, deviceScaleFactor: 1, mobile: true });
  await fresh();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.screenshot({ path: `${screenshots}/gallery-choice-native-mobile.png` });

  await page.cdp('Emulation.setScriptExecutionDisabled', { value: true });
  await fresh();
  await page.focus('#choice-enabled'); await page.press('#choice-enabled', 'Space');
  await page.focus('#choice-channels-choice-1'); await page.press('#choice-channels-choice-1', 'Space');
  await saved('#native-choice-form', true);
  receipt({ channels: [], density: 'compact', enabled: false },
    { channel_count: 0, density: 'compact', email_selected: false, enabled: false, push_selected: false });
  await fresh(); await saved('#native-choice-readonly-form', true);
  receipt({ channels: ['email'], density: 'comfortable', enabled: true },
    { channel_count: 1, density: 'comfortable', email_selected: true, enabled: true, push_selected: false });
  console.log(JSON.stringify({ ok: true, semantics, keyboard: sequence, receipts, forgedStatus: forged.status, noJs: true }));
} finally {
  db.close();
  await page.cdp('Emulation.setScriptExecutionDisabled', { value: false });
}
