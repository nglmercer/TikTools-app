#!/usr/bin/env bun
/**
 * Prepare an isolated host for manual two-PC HTTPS/WSS acceptance.
 *
 * Port of the retired `scripts/collaboration-lab.py`. Usage:
 *   bun scripts/collaboration-lab.ts --directory <lab> prepare [--address A] [--bind IP] [--port N]
 *   bun scripts/collaboration-lab.ts --directory <lab> <host|desktop|configure|status|check|invite|requests|stop>
 *   bun scripts/collaboration-lab.ts --directory <lab> approve <requestId>
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync, chmodSync } from 'node:fs';
import https from 'node:https';
import { extname, dirname, join, resolve } from 'node:path';
import { homedir } from 'node:os';
import { isIP } from 'node:net';

const SCRIPT_DIR = dirname(new URL(import.meta.url).pathname);
const REPO_ROOT = resolve(SCRIPT_DIR, '..');
const DNS_NAME = /^[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?$/;
const SUBCOMMANDS = ['prepare', 'host', 'desktop', 'configure', 'status', 'check', 'invite', 'requests', 'approve', 'stop'] as const;
type Command = (typeof SUBCOMMANDS)[number];

interface Args {
  directory: string;
  binary: string;
  command: Command;
  positional: string | undefined;
  address: string;
  bind: string | undefined;
  port: number;
}

type Json = Record<string, unknown>;

function fail(message: string): never {
  throw new Error(message);
}

function parseArgs(argv: string[]): Args {
  const flags = new Map<string, string>();
  const positionals: string[] = [];
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]!;
    if (token.startsWith('--')) {
      const value = argv[index + 1];
      if (value === undefined || value.startsWith('--')) fail(`${token} requires a value`);
      flags.set(token, value);
      index += 1;
    } else {
      positionals.push(token);
    }
  }
  const directory = flags.get('--directory') ?? fail('--directory is required');
  const command = positionals[0] ?? fail('a command is required');
  if (!(SUBCOMMANDS as readonly string[]).includes(command)) fail(`unknown command: ${command}`);
  const port = Number(flags.get('--port') ?? '8788');
  return {
    directory,
    binary: flags.get('--binary') ?? join(REPO_ROOT, 'target/debug/tiktools'),
    command: command as Command,
    positional: positionals[1],
    address: flags.get('--address') ?? '127.0.0.1',
    bind: flags.get('--bind'),
    port,
  };
}

function expandHome(path: string): string {
  return path.startsWith('~/') ? join(homedir(), path.slice(2)) : path;
}

function save(path: string, value: unknown, compact = false): void {
  writeFileSync(path, `${JSON.stringify(value, null, compact ? undefined : 2)}\n`, { flag: 'wx', mode: 0o600 });
}

function environment(lab: string): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...process.env };
  env.TIKTOOLS_HOME = lab;
  env.TIKTOOLS_IPC_NAME = `two-pc-${createHash('sha256').update(lab).digest('hex').slice(0, 20)}`;
  const directories: Record<string, string> = {
    DATA: 'data', PLUGINS: 'plugins', PLUGIN_DATA: 'plugin-data', BUILTIN_PLUGINS: 'builtins',
    DEV_PLUGINS: 'dev-plugins', LOG: 'logs', TEMP: 'temp', PLUGIN_STORE: 'plugin-store',
  };
  for (const [key, directory] of Object.entries(directories)) {
    const path = join(lab, directory);
    mkdirSync(path, { recursive: true });
    env[`TIKTOOLS_${key}_DIR`] = path;
  }
  env.TIKTOOLS_CLOUD_ORIGIN = '';
  delete env.TIKTOOLS_SESSION_COOKIE;
  return env;
}

function rpc(binary: string, env: NodeJS.ProcessEnv, method: string, params: Json = {}): unknown {
  const result = spawnSync(binary, ['--json', 'rpc', method, JSON.stringify(params)], {
    env, encoding: 'utf8', timeout: 20_000,
  });
  if (result.status !== 0) fail(`${method} failed: ${(result.stderr ?? '').trim()}`);
  return JSON.parse(result.stdout);
}

function sleepMs(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function openssl(args: string[]): void {
  const result = spawnSync('openssl', args, { encoding: 'utf8' });
  if (result.status !== 0) fail(`openssl ${args[0]} failed: ${(result.stderr ?? '').trim()}`);
}

function prepare(args: Args, lab: string): void {
  if (readdirSync(lab).length > 0) fail('prepare requires an empty, dedicated lab directory');
  const { address } = args;
  let san: string;
  if (isIP(address)) {
    san = `IP:${address}`;
  } else {
    if (!DNS_NAME.test(address)) fail('address must be a hostname or an IP, without a scheme or port');
    san = `DNS:${address}`;
  }
  const bind = args.bind ?? address;
  if (!isIP(bind)) fail('bind must be an IP address; it is required for DNS names');
  if (!Number.isInteger(args.port) || args.port < 1 || args.port > 65535) fail('port must be between 1 and 65535');

  const cert = join(lab, 'certificate.pem');
  const key = join(lab, 'private-key.pem');
  openssl([
    'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-noenc', '-days', '7',
    '-subj', `/CN=${address}`, '-addext', `subjectAltName=${san}`, '-keyout', key, '-out', cert,
  ]);
  chmodSync(key, 0o600);
  const host = address.includes(':') ? `[${address}]` : address;
  const bindHost = isIP(bind) === 6 ? `[${bind}]` : bind;
  save(join(lab, 'remote-config.json'), {
    enabled: true,
    bindAddress: `${bindHost}:${args.port}`,
    advertisedOrigin: `https://${host}:${args.port}`,
    allowedOrigins: [],
    tlsCertificate: cert,
    tlsPrivateKey: key,
    tlsProxy: false,
  });
  save(join(lab, 'invitation-grant.json'), {
    grant: {
      capabilities: ['live.read', 'workflows.read', 'workflows.toggle', 'points.read', 'points.adjust', 'commands.read'],
      resources: { workflow: ['two-pc-lab'], viewer: ['two-pc-lab'] },
      maxPointsDelta: 10,
    },
  });
  console.log(`Prepared ${lab}; certificate expires in 7 days.`);
  console.log(`Origin: https://${host}:${args.port}`);
  openssl(['x509', '-in', cert, '-noout', '-fingerprint', '-sha256']);
}

/** Fetches the gateway info endpoint, trusting only this lab's certificate and ignoring proxies. */
function checkGateway(origin: string, certificate: string): Promise<unknown> {
  return new Promise((resolvePromise, reject) => {
    const request = https.get(`${origin}/remote/v1/info`, { ca: readFileSync(certificate), timeout: 10_000 }, (response) => {
      let body = '';
      response.setEncoding('utf8');
      response.on('data', (chunk: string) => { body += chunk; });
      response.on('end', () => {
        try {
          resolvePromise(JSON.parse(body));
        } catch (error) {
          reject(error);
        }
      });
    });
    request.on('timeout', () => request.destroy(new Error('gateway check timed out')));
    request.on('error', reject);
  });
}

async function main(): Promise<void> {
  process.umask(0o077);
  const args = parseArgs(process.argv.slice(2));
  const lab = resolve(expandHome(args.directory));
  mkdirSync(lab, { recursive: true });

  if (args.command === 'prepare') {
    prepare(args, lab);
    return;
  }

  const config = JSON.parse(readFileSync(join(lab, 'remote-config.json'), 'utf8')) as Json;
  const binary = resolve(expandHome(args.binary));
  const env = environment(lab);

  switch (args.command) {
    case 'host': {
      const status = spawnSync(binary, ['host', '--ipc'], { env, cwd: lab, stdio: 'inherit' }).status;
      process.exit(status ?? 1);
      break;
    }
    case 'desktop': {
      const desktop = join(dirname(binary), `tiktools-desktop${extname(binary)}`);
      const status = spawnSync(desktop, [], { env, cwd: lab, stdio: 'inherit' }).status;
      process.exit(status ?? 1);
      break;
    }
    case 'configure': {
      rpc(binary, env, 'collaboration.configure', config);
      for (let attempt = 0; attempt < 30; attempt += 1) {
        const status = rpc(binary, env, 'collaboration.status') as { gateway: { running: boolean } };
        if (status.gateway.running) {
          console.log(JSON.stringify(status, null, 2));
          return;
        }
        sleepMs(200);
      }
      fail('gateway did not start; inspect status');
      break;
    }
    case 'check': {
      const info = await checkGateway(String(config.advertisedOrigin), join(lab, 'certificate.pem'));
      console.log(JSON.stringify(info, null, 2));
      return;
    }
    case 'invite': {
      const target = join(lab, `invitation-${Date.now()}-${process.pid}.json`);
      const grant = JSON.parse(readFileSync(join(lab, 'invitation-grant.json'), 'utf8')) as Json;
      const value = rpc(binary, env, 'collaboration.invites.create', grant);
      save(target, value, true);
      console.log(`Private one-use invitation saved to ${target}; transfer privately to PC B.`);
      return;
    }
    case 'approve': {
      const id = args.positional ?? fail('approve requires a request id');
      console.log(JSON.stringify(rpc(binary, env, 'collaboration.requests.approve', { id }), null, 2));
      return;
    }
    case 'requests':
      console.log(JSON.stringify(rpc(binary, env, 'collaboration.requests.list'), null, 2));
      return;
    case 'stop':
      console.log(JSON.stringify(rpc(binary, env, 'system.shutdown'), null, 2));
      return;
    case 'status':
      console.log(JSON.stringify(rpc(binary, env, 'collaboration.status'), null, 2));
      return;
    default:
      fail(`unhandled command: ${args.command}`);
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`collaboration lab: ${message}`);
  process.exit(1);
});

export { existsSync };
