import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';

/** Real browser component conformance, explicitly NOT Native/backend integration.
 * The page is generated from locked typed fixtures and installs labeled simulator adapters.
 */
export async function runComponentPortProof(page, { origin, screenshots }) {
  await page.cdp('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1100, deviceScaleFactor: 1, mobile: false });
  await page.goto(`${origin}/`);
  await page.waitForFunction(() => Boolean(globalThis.componentProof));
  const initial = await page.evaluate(() => ({
    components: [...document.querySelectorAll('[data-cui-component]')].map(node => node.getAttribute('data-cui-component')),
    duplicateIds: [...document.querySelectorAll('[id]')].map(node => node.id).filter((id, i, all) => all.indexOf(id) !== i),
    closedDialogs: [...document.querySelectorAll('dialog')].every(node => !node.open && getComputedStyle(node).display === 'none'),
    hiddenSelection: getComputedStyle(document.querySelector('[data-cui-file-upload-local-selection]')).display === 'none',
    overflow: document.documentElement.scrollWidth > innerWidth + 1,
  }));
  for (const name of ['modal', 'drawer', 'popover', 'command-menu', 'confirm-dialog', 'date-calendar', 'date-picker', 'file-upload', 'data-viewport']) assert.ok(initial.components.includes(name), name);
  assert.deepEqual(initial.duplicateIds, []); assert.equal(initial.closedDialogs, true); assert.equal(initial.hiddenSelection, true); assert.equal(initial.overflow, false);

  for (const name of ['modal', 'drawer']) {
    const selector = `[data-cui-component="${name}"]`;
    await page.click(`${selector} [data-cui-modal-trigger]`);
    assert.equal(await page.evaluate(name => document.querySelector(`[data-cui-component="${name}"] dialog`).open, name), true);
    const focusInside = await page.evaluate(name => document.querySelector(`[data-cui-component="${name}"] dialog`).contains(document.activeElement), name);
    assert.equal(focusInside, true, `${name}: native focus containment`);
    await page.keyboard.press('Escape');
    await page.waitForFunction(name => !document.querySelector(`[data-cui-component="${name}"] dialog`).open, name);
    assert.equal(await page.evaluate(name => document.activeElement === document.querySelector(`[data-cui-component="${name}"] [data-cui-modal-trigger]`), name), true);
  }

  await page.click('[data-cui-component="popover"] summary');
  assert.equal(await page.evaluate(() => document.querySelector('[data-cui-component="popover"] details')?.open ?? document.querySelector('details[data-cui-component="popover"]').open), true);
  await page.keyboard.press('Escape');

  await page.click('[data-cui-command-trigger]');
  await page.waitForFunction(() => document.querySelector('[data-cui-command-dialog]').open);
  await page.evaluate(() => { const input=document.querySelector('[data-cui-command-search]');input.value='no such illustrative command';input.dispatchEvent(new Event('input',{bubbles:true})); });
  assert.equal(await page.evaluate(() => document.querySelector('[data-cui-command-empty]').hidden), false);
  await page.evaluate(() => { const input=document.querySelector('[data-cui-command-search]');input.value='';input.dispatchEvent(new Event('input',{bubbles:true})); });
  await page.keyboard.press('ArrowDown');
  assert.ok(await page.evaluate(() => document.querySelector('[data-cui-command-search]').getAttribute('aria-activedescendant')));
  await page.keyboard.press('Escape');

  await page.click('[data-cui-confirm-trigger]');
  await page.click('[data-cui-confirm-cancel]');
  assert.equal(await page.evaluate(() => componentProof.state.confirmationIntents), 0);
  await page.evaluate(() => componentProof.failNext('Confirmation'));
  await page.click('[data-cui-confirm-trigger]'); await page.click('[data-cui-confirm-accept]');
  await page.waitForFunction(() => componentProof.state.failures.includes('Simulated confirmation adapter failure'));
  assert.ok(await page.evaluate(() => { const feedback=document.querySelector('[data-cui-confirm-feedback]');return feedback && !feedback.hidden && feedback.textContent.trim() && getComputedStyle(feedback).display !== 'none'; }), 'confirmation rejection is readable before retry');
  assert.equal(await page.evaluate(() => componentProof.state.confirmationIntents), 0);
  // Recovery may keep the dialog open or close it; both preserve original submit intent.
  if (!(await page.evaluate(() => document.querySelector('[data-cui-confirm-dialog]').open))) await page.click('[data-cui-confirm-trigger]');
  await page.click('[data-cui-confirm-accept]');
  await page.waitForFunction(() => componentProof.state.confirmationIntents === 1);
  if (await page.evaluate(() => document.querySelector('[data-cui-confirm-dialog]').open)) await page.keyboard.press('Escape');

  const calendarChoice = await page.evaluate(() => document.querySelector('[data-proof-component="date-calendar"] [data-cui-date-day]:not(:disabled)').getAttribute('data-cui-date-value'));
  await page.click(`[data-proof-component="date-calendar"] [data-cui-date-value="${calendarChoice}"]`);
  await page.waitForFunction(() => componentProof.state.dates.some(event => event.name === 'cui:date-calendar-changed'));
  const draft = await page.evaluate(() => { const input=document.querySelector('[data-proof-component="date-picker"] input[type="date"]');return { id:input.id, before:input.value }; });
  assert.ok(draft.id, 'native date draft fallback is present and labeled');
  const pickerChoice = await page.evaluate(() => {
    const root=document.querySelector('[data-proof-component="date-picker"]');
    const details=root.querySelector('details');if(details)details.open=true;
    return [...root.querySelectorAll('[data-cui-date-day]:not(:disabled)')].find(node=>node.getAttribute('data-cui-date-value')!==root.querySelector('input[type="date"]').value)?.getAttribute('data-cui-date-value');
  });
  assert.ok(pickerChoice);
  await page.click(`[data-proof-component="date-picker"] [data-cui-date-value="${pickerChoice}"]`);
  await page.waitForFunction(value => document.querySelector('[data-proof-component="date-picker"] input[type="date"]').value === value, pickerChoice);
  assert.ok(await page.evaluate(() => componentProof.state.dates.some(event=>event.name==='cui:date-picker-changed')));

  await page.evaluate(() => componentProof.failNext('Selection'));
  await selectIllustrativeFile(page, 'illustrative-failure.txt');
  await page.waitForFunction(() => !document.querySelector('[data-cui-file-upload-local-feedback]').hidden);
  const serverRows = await page.evaluate(() => document.querySelector('.cui-file-upload__files')?.innerHTML ?? null);
  assert.ok(serverRows, 'fixture includes authoritative server-owned file rows');
  await selectIllustrativeFile(page, 'illustrative-retry.txt');
  await page.waitForFunction(() => document.querySelector('[data-cui-file-upload-local-feedback]').hidden);
  assert.equal(await page.evaluate(() => document.querySelector('.cui-file-upload__files')?.innerHTML ?? null), serverRows, 'selection adapter does not fabricate server-owned upload completion');
  await page.evaluate(() => document.querySelector('[data-cui-file-upload-input]').dispatchEvent(new Event('cancel', { bubbles:true })));
  assert.equal(await page.evaluate(() => componentProof.state.chooserCancellations), 1);

  const windowed = '[data-proof-component="windowed-viewport"] [data-cui-scroll]';
  await page.evaluate(selector => { const node=document.querySelector(selector);node.scrollTop=32;node.dispatchEvent(new Event('scroll')); }, windowed);
  await page.waitForFunction(() => componentProof.state.windows.length > 0);
  await page.evaluate(selector => { componentProof.failNext('Window');const node=document.querySelector(selector);node.scrollTop=1024; }, windowed);
  await page.waitForFunction(() => componentProof.state.failures.includes('Simulated window adapter failure'));
  assert.equal(await page.evaluate(() => document.querySelector('[data-proof-component="windowed-viewport"] [data-cui-viewport-status]').hidden), false);
  await page.evaluate(selector => document.querySelector(selector).dispatchEvent(new Event('scroll')), windowed);
  await page.waitForFunction(() => document.querySelector('[data-proof-component="windowed-viewport"] [data-cui-viewport-status]').hidden);
  const navigation = '[data-proof-component="data-viewport"] [data-cui-viewport-navigation]';
  await page.evaluate(selector => { componentProof.failNext('Navigation');document.querySelector(selector).addEventListener('click',event=>event.preventDefault(),{once:true}); }, navigation);
  await page.evaluate(selector => document.querySelector(selector).scrollIntoView({block:'center'}), navigation);
  await page.click(navigation);
  await page.waitForFunction(() => componentProof.state.failures.includes('Simulated navigation adapter failure'));
  await page.evaluate(selector => document.querySelector(selector).addEventListener('click',event=>event.preventDefault(),{once:true}), navigation);
  await page.click(navigation);
  await page.waitForFunction(() => componentProof.state.navigation.length === 1);
  assert.ok(await page.evaluate(() => componentProof.state.navigation[0].href.includes('page=2')));
  const requests = await page.evaluate(() => componentProof.state.windows);
  for (const request of requests) assert.ok(Number.isSafeInteger(request.start) && request.start >= 0 && request.start < request.end && request.end <= request.total);
  if (screenshots?.desktop) await screenshot(page, screenshots.desktop);

  await page.cdp('Emulation.setDeviceMetricsOverride', { width:390, height:844, deviceScaleFactor:1, mobile:true });
  await page.evaluate(() => window.scrollTo(0,0));
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, 'mobile page overflow');
  if (screenshots?.mobile) await screenshot(page, screenshots.mobile);
  await page.evaluate(() => componentProof.cleanup());
  assert.equal(await page.evaluate(() => document.querySelector('[data-cui-file-upload-local-selection]').hidden), true);

  await page.cdp('Emulation.setScriptExecutionDisabled', { value:true });
  try {
    await page.goto(`${origin}/?javascript=disabled`);
    const fallback = await page.evaluate(() => ({
      fileInput: document.querySelector('input[type="file"]')?.disabled === false,
      dateInput: Boolean(document.querySelector('input[type="date"]')),
      confirmForm: Boolean(document.querySelector('[data-cui-confirm-trigger]').form),
      links: [...document.querySelectorAll('[data-cui-modal-trigger],[data-cui-command-trigger]')].every(node => node.getAttribute('href')?.startsWith('/')),
      hiddenSelection: getComputedStyle(document.querySelector('[data-cui-file-upload-local-selection]')).display === 'none',
      overflow: document.documentElement.scrollWidth > innerWidth + 1,
    }));
    assert.deepEqual(fallback, {fileInput:true,dateInput:true,confirmForm:true,links:true,hiddenSelection:true,overflow:false});
  } finally { await page.cdp('Emulation.setScriptExecutionDisabled', { value:false }); }
  return { passed:true, scope:'component conformance with labeled simulator adapters; not Native/backend integration', components:9, browserFiles:'synthetic illustrative File references', noJavaScript:true, viewports:['1440×1100','390×844'] };
}
async function selectIllustrativeFile(page, name) {
  await page.evaluate(name => { const input=document.querySelector('[data-cui-file-upload-input]');const transfer=new DataTransfer();transfer.items.add(new File(['Explicit simulator fixture'],name,{type:'text/plain'}));input.files=transfer.files;input.dispatchEvent(new Event('change',{bubbles:true})); }, name);
}
async function screenshot(page, path) { const image=await page.cdp('Page.captureScreenshot',{format:'png'});await writeFile(path,Buffer.from(image.data,'base64')); }
