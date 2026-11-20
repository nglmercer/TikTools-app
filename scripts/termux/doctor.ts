#!/usr/bin/env node
// Termux environment report and standalone CLI runtime checks.
// Replaces scripts/termux/doctor.sh.
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { commandPath, exitWith, repoRoot, run } from './common.ts';

console.log('TikTools Termux Doctor');
console.log(`${run('uname', ['-s']).stdout?.trim()} ${run('uname', ['-m']).stdout?.trim()}`);
for (const tool of ['rustc', 'cargo', 'clang', 'cmake', 'pkg-config', 'protoc']) {
  const found = commandPath(tool);
  console.log(found ? `${tool}: ${found}` : `${tool}: unavailable`);
}
if (commandPath('rustc')) {
  process.stdout.write(run('rustc', ['-vV']).stdout ?? '');
}
process.stdout.write(run('df', ['-h', '.']).stdout ?? '');
console.log('TikTok network: not tested; desktop WebView: excluded from CLI');

const cli = resolve(process.env.TIKTOOLS_BIN || join(repoRoot, 'target/debug/tiktools'));
const built = (() => {
  try {
    return run('test', ['-x', cli]).status === 0;
  } catch {
    return false;
  }
})();
if (!built) {
  console.log('Runtime checks: skipped (build cargo build -p tiktools-cli --locked first)');
  process.exit(1);
}

const scratch = mkdtempSync(join(process.env.TMPDIR || process.env.HOME || tmpdir(), 'tt-doctor.'));
try {
  const env: NodeJS.ProcessEnv = { ...process.env, TIKTOOLS_HOME: join(scratch, 'home') };
  for (const item of ['DATA', 'PLUGINS', 'PLUGIN_DATA', 'LOG', 'TEMP', 'BUILTIN_PLUGINS', 'DEV_PLUGINS']) {
    env[`TIKTOOLS_${item}_DIR`] = join(scratch, item);
  }
  for (const args of [
    ['--standalone', '--json', 'system', 'info'],
    ['--standalone', '--json', 'system', 'doctor'],
  ]) {
    const result = run(cli, args, { env, inherit: true, cwd: scratch });
    if (result.status !== 0) exitWith(result);
  }
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
