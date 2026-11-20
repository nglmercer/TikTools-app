// Headless agent workspace for manual UI checks. Cross-platform (Bun).
//
// The person signs in to the desktop app with the account printed by `up`.
// The agent then drives a headless workspace on the same account with `do`,
// and the person checks the desktop UI by hand. Nothing here touches the UI.
//
//   bun scripts/e2e/agent-workspace.ts up [--email E --password P]   create or reuse an account, start and sign in a headless workspace
//   bun scripts/e2e/agent-workspace.ts do <tiktools args...>       run one CLI command on the workspace, then sync now
//   bun scripts/e2e/agent-workspace.ts status                       show account, workspace and the last sync
//   bun scripts/e2e/agent-workspace.ts down                         stop the workspace (keeps the account)
//
// Env: TT_BIN (CLI), TT_CLOUD_ORIGIN (default http://127.0.0.1:3000), TT_DEV_PLUGINS (default .dev-plugins)
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { randomBytes } from 'node:crypto';

const root = resolve(import.meta.dir, '..', '..');
const exe = process.platform === 'win32' ? 'tiktools.exe' : 'tiktools';
const bin = process.env.TT_BIN ?? join(root, 'target', 'debug', exe);
const origin = process.env.TT_CLOUD_ORIGIN ?? 'http://127.0.0.1:3000';
const pluginsDir = process.env.TT_DEV_PLUGINS ?? join(root, '.dev-plugins');
const stateDir = join(tmpdir(), 'tt-agent-workspace');
const statePath = join(stateDir, 'state.json');

type State = { email: string; password: string; name: string; home: string; pid: number; origin: string };

function fail(message: string): never {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

function readState(): State {
  if (!existsSync(statePath)) fail('no workspace running; run "up" first');
  return JSON.parse(readFileSync(statePath, 'utf8')) as State;
}

function writeState(state: State): void {
  mkdirSync(stateDir, { recursive: true });
  writeFileSync(statePath, JSON.stringify(state, null, 2));
}

function envFor(state: State): NodeJS.ProcessEnv {
  return { ...process.env, TIKTOOLS_HOME: state.home, TIKTOOLS_IPC_NAME: state.name };
}

function cli(state: State, args: string[]): Promise<{ code: number; out: string; err: string }> {
  return new Promise((done) => {
    const child = spawn(bin, args, { env: envFor(state) });
    let out = '';
    let err = '';
    child.stdout.on('data', (chunk) => (out += chunk));
    child.stderr.on('data', (chunk) => (err += chunk));
    child.on('close', (code) => done({ code: code ?? 1, out, err }));
  });
}

async function cliJson(state: State, args: string[]): Promise<any> {
  const result = await cli(state, args);
  if (result.code !== 0) fail(`tiktools ${args.join(' ')}: ${result.err || result.out}`);
  return JSON.parse(result.out);
}

async function signInAndCreate(email: string, password: string, create: boolean): Promise<void> {
  if (create) {
    const signup = await fetch(`${origin}/api/auth/sign-up/email`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Agent test account', email, password }),
    });
    if (signup.status !== 200) fail(`sign-up failed with HTTP ${signup.status}; is the backend running at ${origin}?`);
  }
}

async function up(args: string[]): Promise<void> {
  if (!existsSync(bin)) fail(`tiktools CLI not found at ${bin}; run: cargo build -p tiktools-cli`);
  const flag = (name: string) => {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : undefined;
  };
  const existing = existsSync(statePath) ? (JSON.parse(readFileSync(statePath, 'utf8')) as State) : null;
  if (existing) fail(`a workspace is already running (pid ${existing.pid}); run "down" first`);

  const email = flag('--email') ?? `agent-${Date.now()}@example.test`;
  const password = flag('--password') ?? `Agent-${randomBytes(12).toString('hex')}!`;
  await signInAndCreate(email, password, flag('--email') === undefined);

  const home = join(stateDir, `home-${Date.now()}`);
  mkdirSync(home, { recursive: true });
  const name = `tt-agent-${randomBytes(4).toString('hex')}`;
  const child = spawn(bin, ['host', '--ipc'], {
    env: { ...process.env, TIKTOOLS_HOME: home, TIKTOOLS_IPC_NAME: name, TIKTOOLS_CLOUD_ORIGIN: origin, TIKTOOLS_DEV_PLUGINS_DIR: pluginsDir },
    detached: true,
    stdio: 'ignore',
  });
  child.unref();
  const state: State = { email, password, name, home, pid: child.pid ?? -1, origin };
  writeState(state);
  for (let i = 0; i < 80; i++) {
    if ((await cli(state, ['system', 'info'])).code === 0) break;
    await Bun.sleep(250);
    if (i === 79) fail('the headless workspace did not start');
  }

  const signin = await fetch(`${origin}/api/auth/sign-in/email`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  if (signin.status !== 200) fail(`sign-in failed with HTTP ${signin.status}`);
  const cookie = signin.headers.getSetCookie().map((c) => c.split(';')[0]).join('; ');
  const begun = await cliJson(state, ['api', 'call', 'cloud.login.begin']);
  const url = new URL(begun.browserUrl);
  const handoff = await fetch(`${origin}/api/v1/desktop/handoff`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: cookie },
    body: JSON.stringify({
      redirectUri: url.searchParams.get('redirectUri'),
      codeChallenge: url.searchParams.get('codeChallenge'),
      state: url.searchParams.get('state'),
    }),
  });
  if (handoff.status !== 200) fail(`handoff failed with HTTP ${handoff.status}`);
  const { callbackUrl } = (await handoff.json()) as { callbackUrl: string };
  const completing = cli(state, ['api', 'call', 'cloud.login.complete']);
  await Bun.sleep(500);
  await fetch(callbackUrl);
  const completed = await completing;
  if (completed.code !== 0) fail(`login completion failed: ${completed.err || completed.out}`);
  await cliJson(state, ['api', 'call', 'sync.enable']);

  console.log('Workspace running and signed in. Sign in to the desktop app with:');
  console.log(`  email:    ${email}`);
  console.log(`  password: ${password}`);
  console.log('Then turn sync on in the desktop app. The agent can now run commands with "do".');
}

async function doCommand(args: string[]): Promise<void> {
  if (args.length === 0) fail('give a tiktools command, for example: do plugin disable hotkeys');
  const state = readState();
  const result = await cli(state, args);
  if (result.code !== 0) fail(`tiktools ${args.join(' ')}: ${result.err || result.out}`);
  console.log(result.out.trim());
  const sync = await cliJson(state, ['api', 'call', 'sync.now']);
  console.log(`sync: pushed=${sync.pushed ?? 0} pulled=${sync.pulled ?? 0} applicationDeferred=${sync.applicationDeferred}`);
  console.log('Check the desktop app now.');
}

async function status(): Promise<void> {
  const state = readState();
  console.log(`account: ${state.email}`);
  console.log(`workspace: ${state.name} (pid ${state.pid}, home ${state.home})`);
  const cloud = await cliJson(state, ['api', 'call', 'cloud.status']);
  const sync = await cliJson(state, ['api', 'call', 'sync.status']);
  console.log(`cloud: ${cloud.state} sync enabled=${sync.enabled} lastSyncedAt=${sync.lastSyncedAt ?? 'never'}`);
}

function down(): void {
  const state = readState();
  try { process.kill(state.pid); } catch { /* already exited */ }
  rmSync(state.home, { recursive: true, force: true });
  rmSync(statePath, { force: true });
  console.log('workspace stopped; the account is kept');
}

const [command, ...rest] = process.argv.slice(2);
switch (command) {
  case 'up': await up(rest); break;
  case 'do': await doCommand(rest); break;
  case 'status': await status(); break;
  case 'down': down(); break;
  default: fail('usage: agent-workspace.ts up|do|status|down');
}
