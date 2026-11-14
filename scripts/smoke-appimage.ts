/** Start an AppImage with disposable data and wait for the actual frontend. */
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { checkAppImageCli, checkAppImageSmokeInstallation, parseAppImageSmokeArgs } from './lib/appimage-smoke.ts';
import { startAppImageProcess } from './lib/appimage-process.ts';

const { imageArg, expectedVersion, mode, requireCli } = parseAppImageSmokeArgs(process.argv.slice(2), process.platform);
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
  // The native single-instance socket uses this directory, independently of TIKTOOLS_HOME.
  XDG_RUNTIME_DIR: join(root, 'runtime'),
  // Mounted mode must never silently become extraction because of an inherited
  // CI setting. Extracted mode verifies startup, not FUSE/updater acceptance.
  APPIMAGE_EXTRACT_AND_RUN: mode === 'extracted' ? '1' : undefined,
};
async function probe(argument: string, ...extra: string[]): Promise<string> {
  const child = startAppImageProcess(image, [argument, ...extra], env, join(root, argument.slice(2)));
  let code: number | null;
  try {
    if (!await child.wait(20_000)) throw new Error(`AppImage ${argument} probe timed out`);
    code = await child.exited;
  } finally { await child.stop(); }
  const [out, err] = await child.output();
  if (code !== 0) throw new Error(`AppImage ${argument} probe failed (exit ${code})\nstdout: ${out.slice(-3000)}\nstderr: ${err.slice(-3000)}`);
  console.log(`AppImage ${argument} (${mode}): ${JSON.stringify(out.trim())}`);
  if (err.trim()) console.error(err.slice(-3000));
  return out.trim();
}
try {
  await mkdir(env.TIKTOOLS_BUILTIN_PLUGINS_DIR, { recursive: true });
  await mkdir(env.TIKTOOLS_HOME, { recursive: true });
  await mkdir(env.XDG_RUNTIME_DIR, { mode: 0o700 });
  const marker = join(env.TIKTOOLS_HOME, 'user-data-marker');
  await writeFile(marker, 'preserve this data');
  if (await probe('--version') !== expectedVersion) throw new Error('AppImage version mismatch');
  checkAppImageSmokeInstallation(await probe('--update-installation-type'), mode);
  if (requireCli) {
    checkAppImageCli(await probe('--cli', '--build-info'), expectedVersion);
    const verification = JSON.parse(await probe('--cli', '--json', 'api', 'verify', '--sandbox', '--timeout', '5')) as { ok?: boolean };
    if (verification.ok !== true) throw new Error('AppImage headless API verification failed');
  }
  const child = startAppImageProcess(image, [], env, join(root, 'frontend'));
  let ready = false;
  try {
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline && !child.ended) {
      const log = await readFile(join(env.TIKTOOLS_HOME, 'logs/tiktools.log'), 'utf8').catch(() => '');
      if (log.includes('WebView frontend-ready')) { ready = true; break; }
      await Bun.sleep(200);
    }
    if (!ready) throw new Error('AppImage did not reach frontend-ready');
    if (await readFile(marker, 'utf8') !== 'preserve this data') throw new Error('isolated profile marker changed');
    console.log(`AppImage ${expectedVersion} reached frontend-ready with an isolated profile (${mode} mode${mode === 'extracted' ? '; FUSE/updater acceptance not tested' : ''})`);
  } finally {
    try {
      if (!ready) {
        console.error(`AppImage wrapper ${child.ended ? `exited with ${await child.exited}` : 'still running'} before frontend-ready was observed`);
        const [stdout, stderr] = await child.output();
        console.error(stdout.slice(-3000));
        console.error(stderr.slice(-3000));
        const log = await readFile(join(env.TIKTOOLS_HOME, 'logs/tiktools.log'), 'utf8').catch(() => '');
        console.error(log.slice(-6000));
      }
    } finally { await child.stop(); }
  }
} finally {
  await rm(root, { recursive: true, force: true });
}
