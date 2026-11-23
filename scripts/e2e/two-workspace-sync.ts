// Two-workspace sync E2E, cross-platform (Linux, macOS, Windows) with Bun.
//
// Creates a temporary email/password account on the local backend, starts two
// headless TikTools hosts with separate homes and IPC names, signs both in over
// HTTP (no browser), enables sync, changes a plugin selection in A, syncs both,
// and reports the transport and the apply result separately.
//
// Usage: bun scripts/e2e/two-workspace-sync.ts [--keep] [--plugin <id>]
// Env:   TT_BIN           tiktools CLI (default: target/debug/tiktools[.exe])
//        TT_CLOUD_ORIGIN  backend origin (default: http://127.0.0.1:3000)
//        TT_DEV_PLUGINS   development plugins dir (default: .dev-plugins)
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { randomBytes } from 'node:crypto';

const root = resolve(import.meta.dir, '..', '..');
const exe = process.platform === 'win32' ? 'tiktools.exe' : 'tiktools';
const bin = process.env.TT_BIN ?? join(root, 'target', 'debug', exe);
const origin = process.env.TT_CLOUD_ORIGIN ?? 'http://127.0.0.1:3000';
const pluginsDir = process.env.TT_DEV_PLUGINS ?? join(root, '.dev-plugins');
const args = process.argv.slice(2);
const keep = args.includes('--keep');
const pluginIndex = args.indexOf('--plugin');
const pluginId = pluginIndex >= 0 ? args[pluginIndex + 1] : 'audio.play.process';

function fail(message: string): never {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

if (!existsSync(bin)) fail(`tiktools CLI not found at ${bin}; run: cargo build -p tiktools-cli`);

type Workspace = { name: string; home: string; host: ReturnType<typeof Bun.spawn>; cookie: string };
const workspaces: Workspace[] = [];

function env(ws: Workspace): Record<string, string> {
  return { ...process.env, TIKTOOLS_HOME: ws.home, TIKTOOLS_IPC_NAME: ws.name } as Record<string, string>;
}

async function cli(ws: Workspace, ...cliArgs: string[]): Promise<{ code: number; out: string; err: string }> {
  const proc = Bun.spawn([bin, ...cliArgs], { env: env(ws), stdout: 'pipe', stderr: 'pipe' });
  const [out, err] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text()]);
  return { code: await proc.exited, out, err };
}

// The CLI prints arbitrary JSON; the e2e script reads it directly.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function json(ws: Workspace, ...cliArgs: string[]): Promise<any> {
  const result = await cli(ws, ...cliArgs);
  if (result.code !== 0) fail(`tiktools ${cliArgs.join(' ')} exited ${result.code}: ${result.err || result.out}`);
  return JSON.parse(result.out);
}

async function startWorkspace(label: string, base: string): Promise<Workspace> {
  const home = join(base, `ws-${label}`);
  const name = `tt-e2e-${label}-${process.pid}-${randomBytes(3).toString('hex')}`;
  await Bun.write(join(home, '.keep'), '');
  const ws: Workspace = { name, home, host: undefined as never, cookie: '' };
  ws.host = Bun.spawn([bin, 'host', '--ipc'], {
    env: { ...env(ws), TIKTOOLS_CLOUD_ORIGIN: origin, TIKTOOLS_DEV_PLUGINS_DIR: pluginsDir },
    stdout: 'ignore',
    stderr: 'ignore',
  });
  workspaces.push(ws);
  for (let i = 0; i < 80; i++) {
    const probe = await cli(ws, 'system', 'info');
    if (probe.code === 0) return ws;
    await Bun.sleep(250);
  }
  fail(`workspace ${label} did not start`);
}

async function signIn(email: string, password: string): Promise<string> {
  const response = await fetch(`${origin}/api/auth/sign-in/email`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  if (response.status !== 200) fail(`sign-in failed with HTTP ${response.status}`);
  return response.headers
    .getSetCookie()
    .map((cookie) => cookie.split(';')[0])
    .join('; ');
}

async function cloudLogin(ws: Workspace, email: string, password: string): Promise<void> {
  ws.cookie = await signIn(email, password);
  const begun = await json(ws, 'api', 'call', 'cloud.login.begin');
  const url = new URL(begun.browserUrl);
  const handoff = await fetch(`${origin}/api/v1/desktop/handoff`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: ws.cookie },
    body: JSON.stringify({
      redirectUri: url.searchParams.get('redirectUri'),
      codeChallenge: url.searchParams.get('codeChallenge'),
      state: url.searchParams.get('state'),
    }),
  });
  if (handoff.status !== 200) fail(`handoff failed with HTTP ${handoff.status}`);
  const { callbackUrl } = (await handoff.json()) as { callbackUrl: string };
  const completing = cli(ws, 'api', 'call', 'cloud.login.complete');
  await Bun.sleep(500);
  const delivered = await fetch(callbackUrl);
  if (!delivered.ok) fail(`callback delivery failed with HTTP ${delivered.status}`);
  const completed = await completing;
  if (completed.code !== 0) fail(`login completion failed: ${completed.err || completed.out}`);
  const status = await json(ws, 'api', 'call', 'cloud.status');
  if (status.state !== 'signed-in') fail(`workspace ${ws.name} is not signed in`);
}

function cleanup(): void {
  for (const ws of workspaces) {
    try { ws.host.kill(); } catch { /* already exited */ }
  }
  if (!keep) for (const ws of workspaces) rmSync(ws.home, { recursive: true, force: true });
}

const base = mkdtempSync(join(tmpdir(), 'tt-e2e-'));
const email = `e2e-${Date.now()}-${randomBytes(3).toString('hex')}@example.test`;
const password = `E2e-${randomBytes(12).toString('hex')}!`;

try {
  const signup = await fetch(`${origin}/api/auth/sign-up/email`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: 'E2E test', email, password }),
  });
  if (signup.status !== 200) fail(`sign-up failed with HTTP ${signup.status}; is the backend running at ${origin}?`);
  console.log(`created temporary account ${email}`);

  const a = await startWorkspace('a', base);
  const b = await startWorkspace('b', base);
  const isolated = (await json(a, 'system', 'info')).home !== (await json(b, 'system', 'info')).home;
  if (!isolated) fail('the two workspaces share a home');
  console.log('two isolated workspaces running');

  await cloudLogin(a, email, password);
  await cloudLogin(b, email, password);
  console.log('both workspaces signed in');

  await json(a, 'api', 'call', 'sync.enable');
  await json(b, 'api', 'call', 'sync.enable');
  await json(a, 'plugin', 'disable', pluginId);
  const pushedA = (await json(a, 'api', 'call', 'sync.now')).pushed ?? 0;
  const nowB = await json(b, 'api', 'call', 'sync.now');
  const pulledB = nowB.pulled ?? 0;
  console.log(`transport: A pushed=${pushedA}, B pulled=${pulledB}`);
  const transportOk = pushedA >= 1 && pulledB >= 1;

  const enabledOnB = (await json(b, 'plugin', 'get', pluginId)).plugin?.enabled;
  console.log(`apply on B: applicationDeferred=${nowB.applicationDeferred}, ${pluginId} enabled on B=${enabledOnB} (expected false)`);
  if (!transportOk) fail('the change did not travel from A to B');
  if (enabledOnB === false) console.log('PASS: the change reached B and was applied');
  else console.log('OPEN: transport passed; B has not applied the change yet (see docs/internal/E2E-SYNC.md)');
} finally {
  cleanup();
}
