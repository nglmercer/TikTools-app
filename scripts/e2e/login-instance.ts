// Sign a TikTools instance in to the local backend without a browser.
// Works for the desktop app (its default control IPC) and for a headless host.
//
//   bun scripts/e2e/login-instance.ts --target desktop [--email E --password P]
//   bun scripts/e2e/login-instance.ts --target headless --name N --home H [--start-host] [--email E --password P]
//
// Without --email/--password a new temporary account is created on the backend.
// Env: TT_BIN (CLI), TT_CLOUD_ORIGIN (default http://127.0.0.1:3000), TT_DEV_PLUGINS (default .dev-plugins)
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { randomBytes } from 'node:crypto';

const root = resolve(import.meta.dir, '..', '..');
const exe = process.platform === 'win32' ? 'tiktools.exe' : 'tiktools';
const bin = process.env.TT_BIN ?? join(root, 'target', 'debug', exe);
const origin = process.env.TT_CLOUD_ORIGIN ?? 'http://127.0.0.1:3000';
const pluginsDir = process.env.TT_DEV_PLUGINS ?? join(root, '.dev-plugins');

const args = process.argv.slice(2);
const flag = (name: string) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};
const target = flag('--target') ?? 'desktop';
const startHost = args.includes('--start-host');
const name = flag('--name');
const home = flag('--home');

function fail(message: string): never {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

// desktop: no overrides, so the CLI uses the app's default IPC and home.
// headless: the IPC name and home must match the host's.
function envFor(): NodeJS.ProcessEnv {
  if (target === 'desktop') {
    const copy = { ...process.env };
    delete copy.TIKTOOLS_IPC_NAME;
    delete copy.TIKTOOLS_HOME;
    return copy;
  }
  if (!name || !home) fail('headless needs --name and --home');
  return { ...process.env, TIKTOOLS_IPC_NAME: name, TIKTOOLS_HOME: home };
}

function cli(cliArgs: string[]): Promise<{ code: number; out: string; err: string }> {
  return new Promise((done) => {
    const child = spawn(bin, cliArgs, { env: envFor() });
    let out = '';
    let err = '';
    child.stdout.on('data', (chunk) => (out += chunk));
    child.stderr.on('data', (chunk) => (err += chunk));
    child.on('close', (code) => done({ code: code ?? 1, out, err }));
  });
}

async function cliJson(cliArgs: string[]): Promise<any> {
  const result = await cli(cliArgs);
  if (result.code !== 0) fail(`tiktools ${cliArgs.join(' ')}: ${result.err || result.out}`);
  return JSON.parse(result.out);
}

async function ensureAccount(): Promise<{ email: string; password: string }> {
  const email = flag('--email') ?? `e2e-${Date.now()}-${randomBytes(3).toString('hex')}@example.test`;
  const password = flag('--password') ?? `E2e-${randomBytes(12).toString('hex')}!`;
  if (flag('--email') === undefined) {
    const signup = await fetch(`${origin}/api/auth/sign-up/email`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'E2E test', email, password }),
    });
    if (signup.status !== 200) fail(`sign-up failed with HTTP ${signup.status}; is the backend running at ${origin}?`);
  }
  return { email, password };
}

async function startHeadless(): Promise<void> {
  if (!name || !home) fail('--start-host needs --name and --home');
  mkdirSync(home, { recursive: true });
  const child = spawn(bin, ['host', '--ipc'], {
    env: { ...envFor(), TIKTOOLS_CLOUD_ORIGIN: origin, TIKTOOLS_DEV_PLUGINS_DIR: pluginsDir },
    detached: true,
    stdio: 'ignore',
  });
  child.unref();
  for (let i = 0; i < 80; i++) {
    if ((await cli(['system', 'info'])).code === 0) return;
    await Bun.sleep(250);
  }
  fail('the headless host did not start');
}

async function login(email: string, password: string): Promise<void> {
  const signin = await fetch(`${origin}/api/auth/sign-in/email`, {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  if (signin.status !== 200) fail(`sign-in failed with HTTP ${signin.status}`);
  const cookie = signin.headers.getSetCookie().map((c) => c.split(';')[0]).join('; ');

  const begun = await cliJson(['api', 'call', 'cloud.login.begin']);
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

  const completing = cli(['api', 'call', 'cloud.login.complete']);
  await Bun.sleep(500);
  await fetch(callbackUrl);
  const completed = await completing;
  if (completed.code !== 0) fail(`login completion failed: ${completed.err || completed.out}`);
}

const { email, password } = await ensureAccount();
if (startHeadless && target === 'headless' && args.includes('--start-host')) await startHeadless();
await login(email, password);
const status = await cliJson(['api', 'call', 'cloud.status']);
if (status.state !== 'signed-in') fail(`${target} is not signed in: ${JSON.stringify(status)}`);
console.log(`${target}: signed-in as ${status.email} (accountId ${status.accountId})`);
console.log(`ACCOUNT ${email}`);
console.log(`PASSWORD ${password}`);
