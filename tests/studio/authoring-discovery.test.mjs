import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, mkdir, readFile, writeFile, rm, realpath } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const serverPath = fileURLToPath(new URL('../../../clanker-studio/src/page-server.mjs', import.meta.url));
const cli = join(root, 'target/debug/clanker-ui');
const exec = promisify(execFile);

test('Native Studio forwards authoring grammar from the real byte-pinned CLI', {
  skip: !existsSync(serverPath) || !existsSync(cli)
    ? 'Local integration requires sibling Studio and a built debug CLI.' : false,
}, async () => {
  const { createPageServer } = await import(pathToFileURL(serverPath));
  const base = await realpath(await mkdtemp(join(tmpdir(), 'cui-authoring-studio-')));
  let studio;
  try {
    const app = join(base, 'app');
    await mkdir(join(app, 'ui/pages'), { recursive: true });
    await writeFile(join(app, 'ui/pages/index.html'), '<cui-page-header title="Discovery fixture" />');
    await writeFile(join(app, 'ui/app.css'), '');
    const lock = join(app, 'ui/ui.lock.json');
    const { cp } = await import('node:fs/promises');
    await cp(join(root, 'packages/vanilla'), join(base, 'package'), { recursive: true });
    await exec(cli, ['lock', '--lock', lock, '--package', '../../package']);
    const digest = 'sha256:' + createHash('sha256').update(await readFile(cli)).digest('hex');
    studio = await createPageServer({
      root: fileURLToPath(new URL('../../../clanker-studio/', import.meta.url)),
      workspace: join(base, 'workspace'), token: 'local-test-only',
      config: {
        sourceRoot: app, name: 'Authoring discovery fixture',
        pageFile: 'ui/pages/index.html', editableFiles: ['ui/pages/index.html', 'ui/app.css'],
        previewUrl: 'http://127.0.0.1:62535/', clankerCli: cli, clankerCliDigest: digest,
      },
      // This tests discovery transport, NOT a Native build or rendered browser.
      nativeObserver: { status: () => ({ status: 'ready' }) },
    });
    await new Promise(resolve => studio.server.listen(0, '127.0.0.1', resolve));
    for (const name of ['data-table', 'filter-bar', 'select-field', 'breadcrumbs', 'pagination']) {
      const manifest = JSON.parse(await readFile(join(root, `packages/vanilla/components/${name}/component.json`), 'utf8'));
      const described = await studio.tools.studio_describe.execute({ component: name });
      assert.deepEqual(described.templateAuthoring, manifest.templateAuthoring);
    }
    const pinned = JSON.parse(await readFile(lock, 'utf8'));
    assert.equal(studio.state().package.digest, pinned.package.digest);
  } finally {
    if (studio) await studio.close();
    await rm(base, { recursive: true, force: true });
  }
});
