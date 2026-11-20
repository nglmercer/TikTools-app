#!/usr/bin/env node
// Standalone CLI smoke test on Termux or any host: IPC ownership, SIGINT/SIGTERM
// shutdown, stale sockets, stdio events and sandbox verification.
// Replaces scripts/termux/smoke.sh, which embedded a Python script. Node built-ins only.
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { closeSync, existsSync, mkdirSync, mkdtempSync, openSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { cargoJobs, repoRoot, run } from './common.ts';

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

if (!process.env.TIKTOOLS_BIN) {
  const build = run('cargo', ['build', '-p', 'tiktools-cli', '--locked'], {
    inherit: true,
    env: { ...process.env, CARGO_BUILD_JOBS: cargoJobs() },
  });
  if (build.status !== 0) process.exit(build.status ?? 1);
  process.env.TIKTOOLS_BIN = join(repoRoot, 'target/debug/tiktools');
}

const cli = resolve(process.env.TIKTOOLS_BIN!);
const scratch = mkdtempSync(join(process.env.TMPDIR || process.env.HOME || tmpdir(), 'tt-'));
const root = scratch;
const env: NodeJS.ProcessEnv = { ...process.env };
delete env.TIKTOOLS_SESSION_COOKIE;
env.TIKTOOLS_HOME = join(root, 'home');
for (const item of ['DATA', 'PLUGINS', 'PLUGIN_DATA', 'LOG', 'TEMP', 'BUILTIN_PLUGINS', 'DEV_PLUGINS']) {
  env[`TIKTOOLS_${item}_DIR`] = join(root, item);
}
const socketPath = join(root, 'home', 'tiktools-control.sock');
const logPath = join(root, 'host.log');
const logFd = openSync(logPath, 'w+');

/** Runs one CLI JSON command and returns the parsed value, failing like the Python original. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any -- the CLI JSON is inspected field by field here
function cliJson(...args: string[]): any {
  const proc = spawnSync(cli, ['--json', ...args], { cwd: root, env, encoding: 'utf8', timeout: 30_000 });
  if (proc.error || proc.status !== 0) {
    throw new Error(`${args}: ${proc.status}: ${proc.stderr}: ${proc.stdout}`);
  }
  const value = JSON.parse(proc.stdout);
  assert(!(value && value.jsonrpc === '2.0' && 'error' in value), JSON.stringify(value));
  return value;
}

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** Resolves with the exit code, or rejects when the deadline passes. */
function waitExit(child: ChildProcess, ms: number): Promise<number> {
  if (child.exitCode !== null) return Promise.resolve(child.exitCode);
  return new Promise((done, reject) => {
    const timer = setTimeout(() => reject(new Error('process did not exit before its deadline')), ms);
    child.once('exit', (code) => {
      clearTimeout(timer);
      done(code ?? 1);
    });
  });
}

let host: ChildProcess | undefined;

async function start(): Promise<void> {
  host = spawn(cli, ['host', '--ipc'], { cwd: root, env, stdio: ['inherit', logFd, logFd] });
  const current = host;
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (current.exitCode !== null) {
      throw new Error(`host exited: ${readFileSync(logPath, 'utf8')}`);
    }
    try {
      const info = cliJson('system', 'info');
      assert(info.pid === current.pid, 'connected to unexpected runtime');
      return;
    } catch {
      await sleep(100);
    }
  }
  throw new Error('IPC startup deadline exceeded');
}

/** Collects a child's stdout and stderr, then waits for it to exit. */
function collect(child: ChildProcess, ms: number): Promise<{ code: number; stdout: string; stderr: string }> {
  let stdout = '';
  let stderr = '';
  child.stdout?.on('data', (chunk) => (stdout += chunk));
  child.stderr?.on('data', (chunk) => (stderr += chunk));
  return waitExit(child, ms).then((code) => ({ code, stdout, stderr }));
}

async function main(): Promise<void> {
  cliJson('--standalone', 'system', 'info');
  await start();
  const current = host!;

  const duplicate = spawnSync(cli, ['host', '--ipc'], { cwd: root, env, encoding: 'utf8', timeout: 10_000 });
  assert(duplicate.status !== 0, 'duplicate host acquired ownership');

  const discoveries: [string, ...string[]][] = [
    ['system', 'health'], ['system', 'doctor'], ['live', 'status'], ['plugin', 'list'],
    ['processor', 'list'], ['processor', 'status'], ['automation', 'list'], ['workflow', 'list'], ['api', 'discover'],
  ];
  for (const args of discoveries) cliJson(...args);
  assert(cliJson('system', 'info').pid === current.pid, 'system info reports a different owner');
  assert((statSync(socketPath).mode & 0o777) === 0o600, 'control socket is not owner-only');

  // Validate the public event inspector, with retries to avoid a subscription/startup race.
  // Only this isolated points store changes.
  const watcher = spawn(cli, ['--json', 'api', 'events', '--topics', 'points.*', '--max-events', '1', '--timeout', '8'], {
    cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'],
  });
  const watched = collect(watcher, 10_000);
  try {
    for (let attempt = 0; attempt < 10; attempt += 1) {
      cliJson('points', 'adjust', 'smoke', '1');
      if (watcher.exitCode !== null) break;
      await sleep(100);
    }
    const output = await watched;
    assert(output.code === 0, output.stderr);
    assert(output.stdout.includes('points.changed') && JSON.stringify(JSON.parse(output.stdout)).includes('points.changed'), output.stdout);
  } finally {
    if (watcher.exitCode === null) {
      watcher.kill('SIGTERM');
    }
  }

  // All mutations below are confined to this throwaway runtime.
  cliJson('globals', 'set', 'termuxSmoke', 'Unicode 😀');
  current.kill('SIGTERM');
  assert((await waitExit(current, 35_000)) === 0, 'SIGTERM shutdown failed');
  assert(!existsSync(socketPath), 'SIGTERM left socket behind');

  const portable = join(root, 'PLUGINS', 'portable-smoke');
  mkdirSync(portable, { recursive: true });
  writeFileSync(join(portable, 'plugin.json'), JSON.stringify({
    schemaVersion: 3, apiVersion: 1, id: 'portable-smoke', name: 'PortableSmoke', version: '1.0.0', runtime: 'napi-vm', entry: 'index.js',
  }));
  writeFileSync(join(portable, 'index.js'), 'export default {call(request, context) { return {summary: "Android", logs: [], intents: [], events: []}; }};');
  await start();
  const second = host!;
  assert(cliJson('globals', 'get', 'termuxSmoke').value === 'Unicode 😀', 'globals did not persist');
  cliJson('plugin', 'enable', 'portable-smoke');
  cliJson('plugin', 'health', 'portable-smoke');
  cliJson('plugin', 'disable', 'portable-smoke');
  second.kill('SIGINT');
  assert((await waitExit(second, 35_000)) === 0, 'SIGINT shutdown failed');
  assert(!existsSync(socketPath), 'SIGINT left socket behind');

  await start();
  const platform = cliJson('system', 'info').platform;
  const android = platform === 'AndroidTermux' || platform === 'AndroidOther';
  if (android) {
    const native = join(root, 'PLUGINS', 'unvalidated-native');
    mkdirSync(native, { recursive: true });
    writeFileSync(join(native, 'plugin.json'), JSON.stringify({
      schemaVersion: 3, id: 'unvalidated-native', name: 'Unvalidated Native', version: '1.0.0', runtime: 'native', entry: 'desktop.so',
    }));
    writeFileSync(join(native, 'desktop.so'), 'not a validated Android library');
  }
  // SIGKILL leaves a stale socket; only the ownership-guarded host may recover it.
  const third = host!;
  third.kill('SIGKILL');
  await waitExit(third, 5_000);
  await start();
  assert(cliJson('globals', 'get', 'termuxSmoke').value === 'Unicode 😀', 'globals lost after SIGKILL');
  if (android) {
    assert(JSON.stringify(cliJson('plugin', 'list')).includes('Android ABI validation'), 'native plugin validation missing');
    const refused = spawnSync(cli, ['--json', 'plugin', 'start', 'unvalidated-native'], { cwd: root, env, encoding: 'utf8', timeout: 10_000 });
    assert(refused.status === 1, refused.stderr);
    assert(JSON.parse(refused.stdout).error.code === 'capability_unavailable', refused.stdout);
  }
  cliJson('api', 'call', 'system.shutdown', '{}', '--confirm');
  assert((await waitExit(host!, 35_000)) === 0, 'system.shutdown did not exit cleanly');

  // Deterministic local event test: stdio processes requests in order and drains
  // their events before reading the next request, with no LIVE session.
  const requests = [
    { jsonrpc: '2.0', id: 1, method: 'points.adjust', params: { uniqueId: 'smoke', delta: 1 } },
    { jsonrpc: '2.0', id: 2, method: 'system.shutdown', params: {} },
  ].map((request) => JSON.stringify(request)).join('\n') + '\n';
  const stdio = spawnSync(cli, ['host', '--stdio', '--events'], { cwd: root, env, input: requests, encoding: 'utf8', timeout: 40_000 });
  assert(stdio.status === 0, stdio.stderr);
  const messages = stdio.stdout.split('\n').filter(Boolean).map((line) => JSON.parse(line));
  assert(messages.some((message) => JSON.stringify(message).includes('points.changed')), JSON.stringify(messages));
  assert(messages.every((message) => !('error' in message)), JSON.stringify(messages));

  // A live stdin pipe must not keep the runtime alive after signal cleanup.
  const idle = spawn(cli, ['host', '--stdio'], { cwd: root, env, stdio: ['pipe', 'pipe', 'pipe'] });
  try {
    let buffered = '';
    const firstLine = new Promise<string>((done, reject) => {
      const timer = setTimeout(() => reject(new Error('stdio startup deadline')), 10_000);
      idle.stdout!.on('data', (chunk) => {
        buffered += chunk;
        const newline = buffered.indexOf('\n');
        if (newline >= 0) {
          clearTimeout(timer);
          done(buffered.slice(0, newline));
        }
      });
    });
    idle.stdin!.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'system.info', params: {} }) + '\n');
    JSON.parse(await firstLine);
    await sleep(300);
    idle.kill('SIGTERM');
    assert((await waitExit(idle, 10_000)) === 0, 'idle stdio signal shutdown failed');
  } finally {
    if (idle.exitCode === null) idle.kill('SIGKILL');
  }

  const report = cliJson('api', 'verify', '--sandbox');
  assert(report.ok === true, JSON.stringify(report));
  console.log('PASS: standalone, IPC, same owner, JSON discovery, SQLite restart, SIGINT/SIGTERM, stale socket, CLI/stdio events, portable VM lifecycle, unsupported native errors, sandbox');
  console.log('SKIPPED: authenticated LIVE, external/native plugins, frontend, desktop');
}

try {
  await main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  if (host && host.exitCode === null) {
    host.kill('SIGTERM');
    await waitExit(host, 35_000).catch(() => host?.kill('SIGKILL'));
  }
  closeSync(logFd);
  rmSync(scratch, { recursive: true, force: true });
}
