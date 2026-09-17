import { createConnection, createServer, type Socket } from 'node:net';
import { homedir } from 'node:os';
import { join } from 'node:path';

export const DEV_HOST = '127.0.0.1';
export const IPC_NAME = 'tiktools-control';
export const DESKTOP_BINARY = 'tiktools-desktop';

export function devUrl(port: number): string {
  return `http://${DEV_HOST}:${port}`;
}

/** Allocate a free TCP port on 127.0.0.1 by binding port 0 once. */
export async function allocateFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.on('error', reject);
    server.listen(0, DEV_HOST, () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : 0;
      server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        if (!port) {
          reject(new Error('could not allocate a free dev port'));
          return;
        }
        resolve(port);
      });
    });
  });
}

function defaultDataRoot(platform: NodeJS.Platform = process.platform): string {
  if (platform === 'win32') {
    return (
      process.env.LOCALAPPDATA || join(process.env.USERPROFILE || homedir(), 'AppData', 'Local')
    );
  }
  if (platform === 'darwin') {
    return join(process.env.HOME || homedir(), 'Library', 'Application Support');
  }
  return process.env.XDG_DATA_HOME || join(process.env.HOME || homedir(), '.local', 'share');
}

function appRoot(): string {
  const override = process.env.TIKTOOLS_HOME;
  if (override && override.length > 0) return override;
  return join(defaultDataRoot(), 'TikTools');
}

function validIpcName(name: string): boolean {
  return (
    name.length > 0 &&
    name.length <= 64 &&
    [...name].every((c) => /[A-Za-z0-9\-_]/.test(c))
  );
}

/** Local IPC endpoint: pipe path on Windows, socket path elsewhere. */
export function ipcEndpoint(platform: NodeJS.Platform = process.platform): string {
  if (platform === 'win32') {
    const override = process.env.TIKTOOLS_IPC_NAME;
    if (override && validIpcName(override)) return `\\\\.\\pipe\\${override}`;
    return `\\\\.\\pipe\\${IPC_NAME}`;
  }
  return join(appRoot(), `${IPC_NAME}.sock`);
}

/**
 * Probe whether a control host already owns the production IPC endpoint.
 * A successful connection means a stale desktop/host is still running.
 */
export async function isControlHostRunning(timeoutMs = 500): Promise<boolean> {
  const path = ipcEndpoint();
  return new Promise((resolve) => {
    let settled = false;
    const done = (value: boolean): void => {
      if (settled) return;
      settled = true;
      resolve(value);
    };
    let socket: Socket;
    try {
      socket = createConnection(path);
    } catch {
      done(false);
      return;
    }
    const timer = setTimeout(() => {
      socket.destroy();
      done(false);
    }, timeoutMs);
    socket.on('connect', () => {
      clearTimeout(timer);
      socket.destroy();
      done(true);
    });
    socket.on('error', () => {
      clearTimeout(timer);
      done(false);
    });
  });
}

/**
 * Parse `tasklist /FO CSV /NH` output for desktop owners. A no-match run
 * prints an INFO line instead of rows.
 */
export function parseTasklistOwners(output: string): number[] {
  const owners: number[] = [];
  for (const line of output.split('\n')) {
    const match = line.match(/^"([^"]+)"\s*,\s*"(\d+)"/);
    if (match && match[1]?.toLowerCase() === `${DESKTOP_BINARY}.exe`) {
      owners.push(Number(match[2]));
    }
  }
  return owners;
}

/**
 * Parse `ps -A -o pid=,args=` output for desktop owners: the desktop
 * binary itself, or a `cargo run` parent currently hosting one. Plain
 * builds (`cargo build -p tiktools-desktop`) and editors with the path
 * open must not match.
 */
export function parsePsOwners(output: string): number[] {
  const owners: number[] = [];
  for (const line of output.split('\n')) {
    const match = line.trim().match(/^(\d+)\s+(.*)$/);
    if (!match) continue;
    const [, pid, args] = match as [string, string, string];
    const first = args.split(/\s+/, 1)[0] ?? '';
    const base = first.split(/[\\/]/).pop() ?? '';
    const isBinary =
      base === DESKTOP_BINARY ||
      base === `${DESKTOP_BINARY}.exe` ||
      base.startsWith(`${DESKTOP_BINARY}.`);
    const isCargoRunHost =
      /(^|[/\\])cargo(\.exe)?\s+run\b/.test(args) && args.includes(DESKTOP_BINARY);
    if (isBinary || isCargoRunHost) owners.push(Number(pid));
  }
  return owners;
}

/**
 * Find PIDs of any running desktop owner outside this launcher. Used
 * alongside the IPC probe: a degraded desktop with a downed IPC server
 * is still a stale owner that must block a new dev session. Probe
 * failures fail open (empty) with the IPC check remaining authoritative.
 */
export function findDesktopOwners(platform: NodeJS.Platform = process.platform): number[] {
  try {
    if (platform === 'win32') {
      const result = Bun.spawnSync({
        cmd: ['tasklist', '/FO', 'CSV', '/NH', '/FI', `IMAGENAME eq ${DESKTOP_BINARY}.exe`],
        stdout: 'pipe',
        stderr: 'ignore',
      });
      if (!result.success) return [];
      return parseTasklistOwners(result.stdout.toString());
    }
    const result = Bun.spawnSync({
      cmd: ['ps', '-A', '-o', 'pid=,args='],
      stdout: 'pipe',
      stderr: 'ignore',
    });
    if (!result.success) return [];
    return parsePsOwners(result.stdout.toString());
  } catch {
    return [];
  }
}

export async function waitForHttp(
  url: string,
  isOwnerAlive: () => boolean,
  attempts = 100,
  delayMs = 100,
): Promise<void> {
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    if (!isOwnerAlive()) {
      throw new Error(
        `Vite dev server exited before becoming ready at ${url}; the port may be taken or the server failed to start`,
      );
    }
    try {
      const response = await fetch(url);
      if (response) {
        await response.body?.cancel().catch(() => {});
        // The owned Vite process must still be alive: another process
        // answering on this port does not count as our dev server.
        if (!isOwnerAlive()) {
          throw new Error(
            `Vite dev server exited while probing ${url}; refusing to reuse another process on that port`,
          );
        }
        return;
      }
    } catch (error) {
      if (error instanceof Error && error.message.includes('refusing to reuse')) throw error;
      // Not up yet; keep waiting within the bounded retry budget.
    }
    await Bun.sleep(delayMs);
  }
  throw new Error(`Vite dev server did not become ready at ${url}`);
}
