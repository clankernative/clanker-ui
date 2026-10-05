import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';

/** Run against the disposable Native gallery, not a fixture-only HTML server.
 * Clipboard writes are stubbed so the proof never overwrites the user's clipboard.
 * DOM replacement and persisted events below are supplemental lifecycle probes,
 * not claims about Native query-patch or actual BFCache admission.
 */
export async function runNativeEnhancementProof(page, { origin, screenshots }) {
  const root = '#native-enhancement-proof';
  const copy = `${root} [data-cui-copy-trigger]`;
  const theme = value => `${root} [data-cui-theme-value="${value}"]`;
  const tip = `${root} [data-cui-tooltip-trigger]`;
  await page.cdp('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1500, deviceScaleFactor: 1, mobile: false });
  await page.goto(`${origin}/`);
  await page.goto(`${origin}/layouts#native-enhancement-proof`);
  await page.waitForSelector(`${root} [data-cui-tooltip-enhanced]`);
  await page.evaluate(() => document.querySelector('#native-enhancement-proof').scrollIntoView({ block: 'start' }));
  const semantics = await page.evaluate(() => {
    const root = document.querySelector('#native-enhancement-proof');
    const input = root.querySelector('[data-cui-copy-source]');
    const trigger = root.querySelector('[data-cui-tooltip-trigger]');
    return { readonly: input.readOnly, label: document.querySelector(`label[for="${input.id}"]`)?.textContent,
      tooltip: document.getElementById(trigger.getAttribute('aria-describedby'))?.getAttribute('role'),
      notifications: root.querySelectorAll('[data-cui-toast]').length,
      otherTheme: document.documentElement.getAttribute('data-theme'),
      hostModules: [...document.scripts].some(script => script.src.includes('/assets/ui/') && script.src.endsWith('/ui-package.js')) };
  });
  assert.equal(semantics.readonly, true); assert.equal(semantics.label, 'Illustrative reference');
  assert.equal(semantics.tooltip, 'tooltip'); assert.equal(semantics.notifications, 2); assert.equal(semantics.hostModules, true);
  if (screenshots?.desktop) await screenshot(page, screenshots.desktop);

  await page.evaluate(() => { window.__cuiCopies = []; navigator.clipboard.writeText = async value => { window.__cuiCopies.push(value); }; });
  await page.click(copy);
  await page.waitForFunction(() => document.querySelector('[data-cui-copy-status]').textContent === document.querySelector('[data-cui-copy-trigger]').getAttribute('data-cui-copy-label-copied'));
  assert.deepEqual(await page.evaluate(() => window.__cuiCopies), ['ILLUSTRATION-42']);
  await page.evaluate(() => { navigator.clipboard.writeText = async () => { throw new Error('Proof-only denial'); }; });
  await page.click(copy);
  await page.waitForFunction(() => document.querySelector('[data-cui-copy-status]').textContent === document.querySelector('[data-cui-copy-trigger]').getAttribute('data-cui-copy-label-failed'));
  assert.equal(await page.evaluate(() => document.querySelector('[data-cui-copy-source]').value), 'ILLUSTRATION-42');

  await page.click(theme('light'));
  assert.equal(await page.evaluate(() => document.getElementById('enhancement-theme-sample').getAttribute('data-cui-theme')), 'light');
  assert.equal(await page.evaluate(() => document.documentElement.getAttribute('data-theme')), semantics.otherTheme);
  await page.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }] });
  await page.click(theme('system'));
  await page.waitForFunction(() => document.getElementById('enhancement-theme-sample').getAttribute('data-cui-theme') === 'light');
  await page.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] });
  await page.waitForFunction(() => document.getElementById('enhancement-theme-sample').getAttribute('data-cui-theme') === 'dark');

  await page.evaluate(() => document.querySelector('[data-cui-tooltip-trigger]').focus());
  await page.waitForFunction(() => document.querySelector('[data-cui-component="tooltip"]').hasAttribute('data-cui-tooltip-open'));
  const placement = await page.evaluate(() => {
    const bubble = document.querySelector('[data-cui-tooltip-bubble]').getBoundingClientRect();
    return { fits: bubble.left >= 0 && bubble.top >= 0 && bubble.right <= innerWidth && bubble.bottom <= innerHeight };
  });
  assert.equal(placement.fits, true);
  await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(() => document.activeElement === document.querySelector('[data-cui-tooltip-trigger]')), true, 'Escape must retain trigger focus');
  assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector('[data-cui-tooltip-bubble]')).visibility), 'hidden');
  await page.evaluate(() => document.activeElement.blur());
  await page.hover(tip);
  await page.waitForFunction(() => document.querySelector('[data-cui-component="tooltip"]').hasAttribute('data-cui-tooltip-open'));
  await page.hover(`${root} h2`);

  await page.click(`${root} [data-cui-toast-dismiss]`);
  assert.equal(await page.evaluate(() => document.querySelectorAll('#native-enhancement-proof [data-cui-toast]').length), 1);
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })));
  assert.equal(await page.evaluate(() => document.querySelectorAll('#native-enhancement-proof [data-cui-toast]').length), 2, 'explicit reset history policy');

  // Exact-root teardown/reinstall and an app-owned DOM replacement, using the
  // actual host-admitted module rather than importing local test source.
  const lifecycle = await page.evaluate(async () => {
    const entry = [...document.scripts].find(script => script.src.endsWith('/ui-package.js'));
    const api = await import(new URL('./clanker-ui.js', entry.src));
    const stopDocument = api.install(document); const idempotent = stopDocument === api.install(document);
    stopDocument(); stopDocument();
    const root = document.querySelector('#native-enhancement-proof');
    const target = document.getElementById('enhancement-theme-sample');
    target.setAttribute('data-cui-theme', 'light');
    const stopThemeProbe = api.install(root);
    target.setAttribute('data-cui-theme', 'dark'); // App writes the same value the switcher applied.
    stopThemeProbe();
    const themePreserved = target.getAttribute('data-cui-theme') === 'dark';
    window.__cuiProofStop = api.install(root); window.__cuiProofApi = api;
    const oldCopy = root.querySelector('[data-cui-component="copy-field"]');
    navigator.clipboard.writeText = () => new Promise(resolve => { window.__cuiResolve = resolve; });
    oldCopy.querySelector('[data-cui-copy-trigger]').click();
    const replacement = oldCopy.cloneNode(true);
    replacement.querySelector('[data-cui-copy-trigger]').disabled = false;
    replacement.querySelector('[data-cui-copy-source]').value = 'REPLACEMENT-REFERENCE';
    replacement.querySelector('[data-cui-copy-status]').textContent = '';
    oldCopy.replaceWith(replacement); window.__cuiResolve();
    await new Promise(resolve => setTimeout(resolve, 0));
    return { idempotent, themePreserved, staleFeedback: replacement.querySelector('[data-cui-copy-status]').textContent };
  });
  assert.equal(lifecycle.idempotent, true); assert.equal(lifecycle.themePreserved, true, 'teardown must preserve even equal-value app writes'); assert.equal(lifecycle.staleFeedback, '');
  await page.evaluate(() => { navigator.clipboard.writeText = async value => { window.__cuiCopies.push(value); }; });
  await page.click(copy);
  await page.waitForFunction(() => document.querySelector('[data-cui-copy-status]').textContent === document.querySelector('[data-cui-copy-trigger]').getAttribute('data-cui-copy-label-copied'));
  assert.equal((await page.evaluate(() => window.__cuiCopies)).at(-1), 'REPLACEMENT-REFERENCE');

  await page.evaluate(() => {
    const tip = document.querySelector('[data-cui-component="tooltip"]'); tip.replaceWith(tip.cloneNode(true));
    const theme = document.querySelector('[data-cui-component="theme-switcher"]'); theme.replaceWith(theme.cloneNode(true));
  });
  await page.waitForSelector(`${root} [data-cui-tooltip-enhanced]`);
  await page.click(theme('light'));
  assert.equal(await page.evaluate(() => document.getElementById('enhancement-theme-sample').getAttribute('data-cui-theme')), 'light');
  await page.evaluate(() => document.querySelector('[data-cui-tooltip-trigger]').focus());
  await page.waitForFunction(() => document.querySelector('[data-cui-component="tooltip"]').hasAttribute('data-cui-tooltip-open'));
  await page.keyboard.press('Escape');

  await page.evaluate(() => {
    const region = document.querySelector('#native-enhancement-proof [data-cui-component="toast"]');
    const timed = region.querySelector('[data-cui-toast]').cloneNode(true);
    timed.id = 'proof-timed-notice'; timed.setAttribute('data-cui-toast-timeout', '1000'); region.append(timed);
  });
  await page.hover('#proof-timed-notice');
  await page.evaluate(() => new Promise(resolve => setTimeout(resolve, 1200)));
  assert.equal(await page.evaluate(() => !!document.getElementById('proof-timed-notice')), true, 'hover pauses expiry');
  await page.hover(`${root} h2`);
  await page.waitForFunction(() => !document.getElementById('proof-timed-notice'), null, { timeout: 2500 });

  const stopped = await page.evaluate(async () => {
    const region = document.querySelector('#native-enhancement-proof [data-cui-component="toast"]');
    const toast = region.querySelector('[data-cui-toast]').cloneNode(true); toast.id = 'proof-stopped-notice'; toast.setAttribute('data-cui-toast-timeout', '1000'); region.append(toast);
    await new Promise(resolve => setTimeout(resolve, 0));
    window.__cuiProofStop(); window.__cuiProofStop();
    await new Promise(resolve => setTimeout(resolve, 1200));
    return { timerCancelled: toast.isConnected, tooltipFallback: getComputedStyle(document.querySelector('[data-cui-tooltip-bubble]')).visibility };
  });
  assert.equal(stopped.timerCancelled, true); assert.equal(stopped.tooltipFallback, 'visible');
  await page.evaluate(() => { document.getElementById('proof-stopped-notice').remove(); window.__cuiProofApi.install(document); });

  await page.cdp('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
  await page.goto(`${origin}/`);
  await page.goto(`${origin}/layouts#native-enhancement-proof`);
  await page.waitForSelector(`${root} [data-cui-tooltip-enhanced]`);
  await page.evaluate(() => document.querySelector('#native-enhancement-proof').scrollIntoView({ block: 'start' }));
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, 'mobile overflow');
  await page.evaluate(() => document.querySelector('[data-cui-tooltip-trigger]').focus());
  assert.equal(await page.evaluate(() => { const r = document.querySelector('[data-cui-tooltip-bubble]').getBoundingClientRect(); return r.left >= 0 && r.right <= innerWidth && r.top >= 0 && r.bottom <= innerHeight; }), true);
  await page.keyboard.press('Escape');
  if (screenshots?.mobile) await screenshot(page, screenshots.mobile);

  await page.cdp('Emulation.setScriptExecutionDisabled', { value: true });
  try {
    await page.goto(`${origin}/`);
    await page.goto(`${origin}/layouts#native-enhancement-proof`);
    await page.waitForSelector('#native-enhancement-proof');
    const fallback = await page.evaluate(() => ({
      selectable: document.querySelector('[data-cui-copy-source]').readOnly && document.querySelector('[data-cui-copy-source]').value === 'ILLUSTRATION-42',
      copyHidden: document.querySelector('[data-cui-copy-trigger]').hidden,
      helpVisible: getComputedStyle(document.querySelector('[data-cui-tooltip-bubble]')).visibility,
      helpPosition: getComputedStyle(document.querySelector('[data-cui-tooltip-bubble]')).position,
      helpTrigger: getComputedStyle(document.querySelector('[data-cui-tooltip-trigger]')).display,
      themeHidden: [...document.querySelectorAll('[data-cui-theme-value]')].every(button => button.hidden),
      toasts: document.querySelectorAll('#native-enhancement-proof [data-cui-toast]').length,
      overflow: document.documentElement.scrollWidth > innerWidth + 1
    }));
    assert.deepEqual(fallback, { selectable: true, copyHidden: true, helpVisible: 'visible', helpPosition: 'static', helpTrigger: 'none', themeHidden: true, toasts: 2, overflow: false });
  } finally { await page.cdp('Emulation.setScriptExecutionDisabled', { value: false }); await page.cdp('Emulation.setEmulatedMedia', { features: [] }); }
  return { passed: true, nativeModules: true, clipboard: 'stubbed, no OS clipboard writes', lifecycle: 'teardown, reinstall, DOM replacement, synthetic persisted event', noJavaScript: true, viewports: ['1440×1500', '390×844'] };
}

async function screenshot(page, path) {
  const image = await page.cdp('Page.captureScreenshot', { format: 'png' });
  await writeFile(path, Buffer.from(image.data, 'base64'));
}
