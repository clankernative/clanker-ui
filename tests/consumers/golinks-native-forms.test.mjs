import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import test from 'node:test';

// Run only against a fresh disposable development instance. Never production.
const origin = process.env.GOLINKS_UI_TEST_ORIGIN;
const cookieFile = process.env.GOLINKS_UI_TEST_COOKIE_FILE;
const decode = (text) => text.replace(/&quot;/g, '"').replace(/&#(?:39|x27);/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
const attributes = (tag) => Object.fromEntries([...tag.matchAll(/([\w:-]+)="([^"]*)"/g)].map(([, key, value]) => [key, decode(value)]));
function form(html, id) {
  const match = [...html.matchAll(/<form\b([^>]*)>([\s\S]*?)<\/form>/g)].find((m) => attributes(m[1]).id === id);
  assert.ok(match, `Missing ${id}`);
  const attrs = attributes(match[1]);
  assert.equal(attrs.method, 'post');
  assert.equal(attrs.action, '/actions');
  const fields = {};
  for (const tag of match[2].matchAll(/<input\b[^>]*>/g)) {
    const input = attributes(tag[0]);
    if (input.name && !/\bdisabled(?:\s|=|>)/.test(tag[0])) fields[input.name] = input.value ?? '';
  }
  for (const textarea of match[2].matchAll(/<textarea\b([^>]*)>([\s\S]*?)<\/textarea>/g)) {
    fields[attributes(textarea[1]).name] = decode(textarea[2]);
  }
  assert.ok(fields._csrf && fields._ticket, 'Host-owned form protection required');
  assert.match(match[2], /class="cui-button cui-button--primary"[^>]*type="submit"/);
  return { fields, markup: match[2] };
}

test('real GoLinks component forms retain native submission, drafts, and edit binding', {
  skip: !origin || !cookieFile ? 'requires disposable GOLINKS_UI_TEST_ORIGIN and GOLINKS_UI_TEST_COOKIE_FILE' : false,
}, async () => {
  const url = new URL(origin);
  assert.ok(['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname), 'Local disposable instance only');
  const cookie = (await readFile(cookieFile, 'utf8')).trim();
  const get = async (path) => {
    const response = await fetch(new URL(path, origin), { headers: { cookie }, redirect: 'manual' });
    assert.equal(response.status, 200);
    return response.text();
  };
  const post = async (fields) => {
    // Deliberately no Datastar headers or browser JS: exercise ordinary HTTP forms.
    const response = await fetch(new URL('/actions', origin), {
      method: 'POST', redirect: 'manual',
      headers: { cookie, origin, 'content-type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams(fields),
    });
    return { status: response.status, location: response.headers.get('location'), html: await response.text() };
  };
  const name = `clanker-native-${randomUUID().slice(0, 8)}`;
  const initial = form(await get('/'), 'create-link');
  assert.equal((initial.markup.match(/data-cui-component="form-field"/g) ?? []).length, 3);
  const created = await post({ ...initial.fields, name, url: 'https://example.com/native-form', description: 'Synthetic native form proof' });
  assert.equal(created.status, 303);
  const directory = await get(created.location);
  const link = [...directory.matchAll(/<a\b([^>]*)>go\/([^<]*)<\/a>/g)].find((m) => decode(m[2]) === name);
  assert.ok(link, 'Created link visible through the real query');
  const path = attributes(link[1]).href;

  const duplicate = form(directory, 'create-link');
  const draft = { name, url: 'https://example.com/preserved-draft', description: 'Rejected <literal> & "quotes"\nsecond line' };
  const rejected = await post({ ...duplicate.fields, ...draft });
  assert.ok(rejected.status >= 400 && rejected.status < 500);
  assert.match(rejected.html, /The command was rejected/);
  const retained = form(rejected.html, 'create-link');
  for (const [key, value] of Object.entries(draft)) assert.equal(retained.fields[key], value);
  const recovered = await post({ ...retained.fields, name: `${name}-retry` });
  assert.equal(recovered.status, 303, 'Rejected native form can be corrected and resubmitted');

  const edit = form(await get(path), 'edit-link');
  assert.equal((edit.markup.match(/data-cui-component="form-field"/g) ?? []).length, 2);
  assert.equal(edit.fields.url, 'https://example.com/native-form');
  const description = 'Edited "quotes" & <markup>\nUnicode: café';
  const edited = await post({ ...edit.fields, url: 'https://example.com/native-edited', description });
  assert.equal(edited.status, 303);
  const updated = form(await get(path), 'edit-link');
  assert.equal(updated.fields.url, 'https://example.com/native-edited');
  assert.equal(updated.fields.description, description);
  assert.match(updated.markup, /Editing revision 2/);
});
