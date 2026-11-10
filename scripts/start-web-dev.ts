import { join } from 'node:path';
import { prepareDevelopmentPlugins, repositoryRoot } from './dev-plugins.ts';
import { allocateFreePort, devUrl, isControlHostRunning, waitForHttp } from './lib/dev-launch.ts';

if (await isControlHostRunning()) throw new Error('A TikTools desktop or control host is already running. Stop it before starting headless development.');

async function run(command: string[]): Promise<void> {
  const process = Bun.spawn({ cmd: command, cwd: repositoryRoot, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit' });
  if (await process.exited !== 0) throw new Error(`${command.join(' ')} failed`);
}

let pluginRoot = process.env.TIKTOOLS_DEV_PLUGINS_DIR;
if (!pluginRoot && process.env.TIKTOOLS_SKIP_DEV_PLUGINS !== '1') pluginRoot = await prepareDevelopmentPlugins();
await run(['node', 'scripts/cargo-with-linker.mjs', 'build', '-p', 'tiktools-cli', '--no-default-features', '--features', 'local-web,cloud-auth,remote-client', '--locked']);
if (await isControlHostRunning()) throw new Error('A TikTools control host started during the build. Stop it and retry.');

const binary = join(repositoryRoot, 'target', 'debug', process.platform === 'win32' ? 'tiktools.exe' : 'tiktools');
const adapterPort = await allocateFreePort();
const webPort = await allocateFreePort();
const adapterUrl = devUrl(adapterPort);
const webUrl = devUrl(webPort);
const children: ReturnType<typeof Bun.spawn>[] = [];
function spawn(cmd: string[], env = process.env) {
  const child = Bun.spawn({ cmd, cwd: repositoryRoot, env, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit' });
  children.push(child);
  return child;
}
function cleanup(): void { for (const child of children) { try { child.kill(); } catch { /* already stopped */ } } }
for (const signal of ['SIGINT', 'SIGTERM'] as const) process.once(signal, () => { cleanup(); process.exitCode = signal === 'SIGINT' ? 130 : 143; });

try {
  const host = spawn([binary, 'host', '--ipc'], { ...process.env, ...(pluginRoot ? { TIKTOOLS_DEV_PLUGINS_DIR: pluginRoot } : {}) });
  for (let attempt = 0; attempt < 200 && !await isControlHostRunning(); attempt++) {
    if (host.exitCode !== null) throw new Error('Headless host exited before becoming ready');
    await Bun.sleep(100);
  }
  if (host.exitCode !== null || !await isControlHostRunning()) throw new Error('Headless host did not become ready');
  const adapter = spawn([binary, 'web', '--assets', join(repositoryRoot, 'src', 'web'), '--port', String(adapterPort), '--join-origin', webUrl]);
  await waitForHttp(adapterUrl, () => adapter.exitCode === null);
  const vite = spawn([process.execPath, 'scripts/task.ts', 'serve'], { ...process.env, TIKTOOLS_WEB_PORT: String(webPort), TIKTOOLS_DEV_WEB_ADAPTER: adapterUrl });
  await waitForHttp(webUrl, () => vite.exitCode === null);
  // optional log, already show message from vite or code message
  //console.log(`\nCtrl+click the one-click join link printed above, or open ${webUrl} and enter the pairing code. Frontend changes reload automatically.\n`);
  const exitCode = await Promise.race(children.map((child) => child.exited));
  if (process.exitCode === undefined) process.exitCode = exitCode || 1;
} finally { cleanup(); }
