/**
 * Real-gateway harness for the OBS widget browser suite.
 *
 * Unlike the render suite (fake server + page hook), these tests boot the
 * actual `event-gateway` binary in an installed-package layout
 * (`<tmp>/event-gateway` + `<tmp>/dist/widgets/{follow,gift}/`), so default
 * widget discovery, origin checks, and credential auth run exactly as in
 * production. Domain events are injected through the plugin frame protocol
 * on the gateway's stdin, the same channel the host uses.
 *
 * Runs under Node (Playwright), never Bun: only node: builtins are used.
 */

import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import net from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..', '..');
const GATEWAY_MANIFEST = join(REPOSITORY_ROOT, 'examples', 'event-gateway-process-plugin', 'Cargo.toml');
const GATEWAY_DEBUG_BINARY = join(
  REPOSITORY_ROOT,
  'examples',
  'event-gateway-process-plugin',
  'target',
  'debug',
  'event-gateway',
);
const GATEWAY_PLUGIN_ID = 'tiktools.event-gateway';
const PROTOCOL_VERSION = 1;

export interface RunningGateway {
  port: number;
  baseUrl: string;
  dataDir: string;
  token: string;
  widgetToken: string;
  sendEvent: (envelope: unknown) => void;
  stop: () => Promise<void>;
}

function runOrThrow(command: string, args: string[], cwd: string): void {
  // spawnSync without shell: no interpolation, no globbing.
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} exited with code ${result.status}`);
  }
}

let binaryBuilt = false;

function ensureGatewayBinary(): string {
  if (binaryBuilt) return GATEWAY_DEBUG_BINARY;
  runOrThrow('node', ['scripts/cargo-with-linker.mjs', 'build', '--manifest-path', GATEWAY_MANIFEST], REPOSITORY_ROOT);
  binaryBuilt = true;
  return GATEWAY_DEBUG_BINARY;
}

function ensureWidgetBundles(): string {
  runOrThrow('bun', ['run', 'build:widgets'], REPOSITORY_ROOT);
  return join(REPOSITORY_ROOT, 'dist', 'widgets');
}

function pickFreePort(): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const server = net.createServer();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === 'object') resolvePort(address.port);
        else reject(new Error('could not pick a free loopback port'));
      });
    });
  });
}

function randomCredential(prefix: string): string {
  return `${prefix}_${randomBytes(32).toString('hex')}`;
}

async function waitForHealth(baseUrl: string, timeoutMs: number): Promise<void> {
  const started = Date.now();
  for (;;) {
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) return;
    } catch {
      // Still starting; fall through to the timeout check.
    }
    if (Date.now() - started > timeoutMs) {
      throw new Error(`gateway at ${baseUrl} never became healthy`);
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
}

export async function startGateway(): Promise<RunningGateway> {
  const binary = ensureGatewayBinary();
  const bundles = ensureWidgetBundles();
  const sandbox = join(tmpdir(), `tiktools-widget-e2e-${process.pid}-${Date.now()}`);
  const layout = join(sandbox, 'layout');
  const dataDir = join(sandbox, 'data');
  await mkdir(join(layout, 'dist'), { recursive: true });
  await mkdir(join(dataDir, GATEWAY_PLUGIN_ID), { recursive: true });

  // Installed-package layout: the binary discovers dist/widgets/ beside
  // itself with no widgetsDir override, exactly like production.
  await cp(binary, join(layout, 'event-gateway'));
  await cp(bundles, join(layout, 'dist', 'widgets'), { recursive: true });

  const port = await pickFreePort();
  const token = randomCredential('ttk_e2e');
  const widgetToken = randomCredential('ttw_e2e');
  await writeFile(
    join(dataDir, GATEWAY_PLUGIN_ID, 'settings.json'),
    JSON.stringify({ port, token, widgetToken }),
    'utf8',
  );

  const child: ChildProcessWithoutNullStreams = spawn(join(layout, 'event-gateway'), [], {
    cwd: layout,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: {
      ...process.env,
      TIKTOOLS_PLUGIN_DATA_DIR: dataDir,
      TIKTOOLS_PLUGIN_ID: GATEWAY_PLUGIN_ID,
    },
  });
  // Frame responses accumulate on stdout; drain so the pipe never blocks.
  child.stdout.resume();
  let stderr = '';
  child.stderr.on('data', (chunk: Buffer) => {
    stderr += chunk.toString('utf8');
  });

  const baseUrl = `http://127.0.0.1:${port}`;
  try {
    await waitForHealth(baseUrl, 15_000);
  } catch (error) {
    child.kill('SIGKILL');
    throw error;
  }

  let nextId = 1;
  const sendEvent = (envelope: unknown): void => {
    const frame = Buffer.from(
      JSON.stringify({
        protocolVersion: PROTOCOL_VERSION,
        id: `e2e-${nextId++}`,
        method: 'call',
        payload: { type: 'event', event: envelope },
      }),
      'utf8',
    );
    const header = Buffer.alloc(4);
    header.writeUInt32LE(frame.length, 0);
    child.stdin.write(Buffer.concat([header, frame]));
  };

  const stop = async (): Promise<void> => {
    child.kill('SIGTERM');
    await new Promise<void>((resolve) => {
      const timer = setTimeout(() => {
        child.kill('SIGKILL');
        resolve();
      }, 5000);
      child.on('exit', () => {
        clearTimeout(timer);
        resolve();
      });
    });
    // Server logs must never carry either credential, even on failure paths.
    if (stderr.includes(token) || stderr.includes(widgetToken)) {
      throw new Error('gateway stderr leaked a test credential');
    }
    await rm(sandbox, { recursive: true, force: true });
  };

  return { port, baseUrl, dataDir, token, widgetToken, sendEvent, stop };
}

/** Reads the staged gateway settings (proves host/plugin file agreement). */
export async function readStagedSettings(dataDir: string): Promise<unknown> {
  const raw = await readFile(join(dataDir, GATEWAY_PLUGIN_ID, 'settings.json'), 'utf8');
  return JSON.parse(raw) as unknown;
}
