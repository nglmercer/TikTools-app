/** Start an AppImage with disposable data and wait for the actual frontend. */
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const [imageArg, expectedVersion, ...extra] = process.argv.slice(2);
if (process.platform !== 'linux' || !imageArg || !expectedVersion || extra.length) {
  throw new Error('usage on Linux: bun scripts/smoke-appimage.ts <AppImage> <expected-version>');
}
const image = resolve(imageArg);
const root = await mkdtemp(join(tmpdir(), 'tiktools-appimage-smoke-'));
const env = {
  ...Object.fromEntries(Object.entries(process.env).filter(([name]) => !name.startsWith('TIKTOOLS_'))),
  TIKTOOLS_HOME: join(root, 'profile'),
  TIKTOOLS_BUILTIN_PLUGINS_DIR: join(root, 'empty-plugins'),
  XDG_CONFIG_HOME: join(root, 'config'),
  XDG_DATA_HOME: join(root, 'data'),
  XDG_CACHE_HOME: join(root, 'cache'),
  XDG_STATE_HOME: join(root, 'state'),
};
async function probe(argument: string): Promise<string> {
  const child = Bun.spawn([image, argument], { env, stdout: 'pipe', stderr: 'pipe' });
  const timer = setTimeout(() => child.kill('SIGKILL'), 20_000);
  const [out, , code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]).finally(() => clearTimeout(timer));
  if (code !== 0) throw new Error(`AppImage ${argument} probe failed`);
  return out.trim();
}
try {
  await mkdir(env.TIKTOOLS_BUILTIN_PLUGINS_DIR, { recursive: true });
  await mkdir(env.TIKTOOLS_HOME, { recursive: true });
  const marker = join(env.TIKTOOLS_HOME, 'user-data-marker');
  await writeFile(marker, 'preserve this data');
  if (await probe('--version') !== expectedVersion) throw new Error('AppImage version mismatch');
  if (await probe('--update-installation-type') !== '"appImage"') throw new Error('AppImage installation detection failed');
  const child = Bun.spawn([image], { env, stdout: 'pipe', stderr: 'pipe' });
  const output = Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text()]);
  let ready = false;
  try {
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline && child.exitCode === null) {
      const log = await readFile(join(env.TIKTOOLS_HOME, 'logs/tiktools.log'), 'utf8').catch(() => '');
      if (log.includes('WebView frontend-ready')) { ready = true; break; }
      await Bun.sleep(200);
    }
    if (!ready) throw new Error('AppImage did not reach frontend-ready');
    if (await readFile(marker, 'utf8') !== 'preserve this data') throw new Error('isolated profile marker changed');
  } finally {
    if (child.exitCode === null) child.kill('SIGINT');
    const killer = setTimeout(() => { if (child.exitCode === null) child.kill('SIGKILL'); }, 5000);
    await child.exited.finally(() => clearTimeout(killer));
    await output;
  }
  console.log(`AppImage ${expectedVersion} reached frontend-ready with an isolated profile`);
} finally {
  await rm(root, { recursive: true, force: true });
}
