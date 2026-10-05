import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import test from 'node:test';

const root = new URL('../../packages/vanilla/', import.meta.url);
const defaults = readFileSync(new URL('theme/default.css', root), 'utf8');
const names = ['activity-feed', 'button-group', 'definition-list', 'disclosure', 'progress-steps', 'segmented-control', 'tabs', 'checkbox-group', 'radio-group', 'toggle', 'copy-field', 'theme-switcher', 'tooltip', 'toast'];

test('ported token defaults preserve every literal fallback and local semantic theming', () => {
  for (const name of names) {
    const manifest = JSON.parse(readFileSync(new URL(`components/${name}/component.json`, root), 'utf8'));
    const css = readFileSync(new URL(manifest.assets.styles, root), 'utf8');
    for (const token of manifest.tokens) {
      // Shared primitives can have a package baseline different from the
      // defensive fallback used when a component is rendered without a theme.
      if (!token.startsWith(`--cui-${name}-`)) continue;
      const declaration = defaults.match(new RegExp(`${token}:\\s*([^;]+);`));
      const starts = [...css.matchAll(new RegExp(`var\\(${token}\\s*[,)]`, 'g'))].map(match => match.index);
      assert.ok(starts.length, `${name}: unused ${token}`);
      const fallbacks = new Set();
      for (const start of starts) {
        let depth = 1, end = start + 4;
        for (; depth; end++) {
          if (css[end] === '(') depth++;
          else if (css[end] === ')') depth--;
          assert.ok(end < css.length, `${token}: malformed fallback`);
        }
        const expression = css.slice(start + 4, end - 1);
        fallbacks.add(expression.slice(expression.indexOf(',') + 1).trim());
      }
      if (!declaration) {
        // Newly described fallback-only tokens retain their CSS-local defaults;
        // discovery does not require copying those values into the root theme.
        assert.ok(manifest.tokenDescriptions?.some(entry => entry.name === token), `${name}: undescribed fallback-only ${token}`);
        assert.ok([...fallbacks].every(value => value.length > 0), `${token}: missing local fallback`);
        continue;
      }
      const value = declaration[1].trim();
      if (fallbacks.size > 1 || [...fallbacks].some(fallback => fallback.startsWith('var('))) {
        assert.equal(value, 'initial', `${token}: resolve contextual fallbacks at the component, not :root`);
      } else {
        assert.equal(value, [...fallbacks][0], `${token}: do not change visual defaults while admitting the contract`);
      }
    }
  }
});
