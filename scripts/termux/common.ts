// Shared helpers for the Termux scripts. Node built-ins only, so these run on
// Termux's Node (>= 22.18, native type stripping) as well as under Bun.
import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { accessSync, constants } from 'node:fs';
import { delimiter, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Repository root: this file lives in scripts/termux/. */
export const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** Equivalent of `command -v <name>`; undefined when the command is not on PATH. */
export function commandPath(name: string): string | undefined {
  for (const dir of (process.env.PATH ?? '').split(delimiter)) {
    if (!dir) continue;
    const candidate = join(dir, name);
    try {
      accessSync(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Keep searching PATH.
    }
  }
  return undefined;
}

export interface RunOptions {
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  /** Stream the child's output to this process instead of capturing it. */
  inherit?: boolean;
  timeoutMs?: number;
}

export function run(command: string, args: string[], options: RunOptions = {}): SpawnSyncReturns<string> {
  return spawnSync(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: options.env ?? process.env,
    encoding: 'utf8',
    stdio: options.inherit ? 'inherit' : 'pipe',
    timeout: options.timeoutMs,
  });
}

/** Exits like a failed shell command would, preserving the child's status. */
export function exitWith(result: SpawnSyncReturns<string>): never {
  process.exit(result.status ?? 1);
}

/** Pins CARGO_BUILD_JOBS unless the caller already chose a value. */
export function cargoJobs(): string {
  return process.env.CARGO_BUILD_JOBS || '2';
}

export function hostTriple(): string {
  const result = run('rustc', ['-vV']);
  const line = (result.stdout ?? '').split('\n').find((text) => text.startsWith('host: '));
  return line ? line.slice('host: '.length).trim() : '';
}
