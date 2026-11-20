#!/usr/bin/env node
// Termux test matrix with an isolated TIKTOOLS_HOME. Replaces scripts/termux/test.sh.
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { cargoJobs, hostTriple, run } from './common.ts';

const scratch = mkdtempSync(join(process.env.TMPDIR || process.env.HOME || tmpdir(), 'tt-tests.'));
let failed = false;

try {
  const env: NodeJS.ProcessEnv = { ...process.env, CARGO_BUILD_JOBS: cargoJobs(), RUST_TEST_THREADS: process.env.RUST_TEST_THREADS || '2' };
  for (const name of ['TIKTOOLS_SESSION_COOKIE', 'TIKTOOLS_DATA_DIR', 'TIKTOOLS_PLUGINS_DIR', 'TIKTOOLS_PLUGIN_DATA_DIR', 'TIKTOOLS_LOG_DIR', 'TIKTOOLS_TEMP_DIR']) {
    delete env[name];
  }
  env.TIKTOOLS_HOME = join(scratch, 'home');
  env.TIKTOOLS_BUILTIN_PLUGINS_DIR = join(scratch, 'builtin');
  env.TIKTOOLS_DEV_PLUGINS_DIR = join(scratch, 'development');

  const cargoTest = (args: string[]) => run('cargo', ['test', ...args, '--locked'], { env, inherit: true });
  const report = (ok: boolean, label: string) => {
    if (ok) {
      console.log(`PASS: ${label}`);
    } else {
      console.error(`FAIL: ${label}`);
      failed = true;
    }
  };

  for (const crate of ['tiktools-cli', 'tiktools-control-api', 'tiktools-client', 'tiktools-core', 'tiktools-tiktok']) {
    const result = cargoTest(['-p', crate]);
    if (result.status === 0) {
      console.log(`PASS: ${crate}`);
    } else {
      console.error(`FAIL: ${crate}`);
      failed = true;
    }
  }
  report(cargoTest(['-p', 'tiktools-core', '--features', 'persistence,native-tiktok,http']).status === 0, 'core with CLI capabilities');
  report(cargoTest(['-p', 'tiktools-plugin-sdk', '--features', 'http-client']).status === 0, 'plugin SDK and native TLS initialization');

  if (hostTriple().includes('android')) {
    report(
      cargoTest(['-p', 'tiktools-plugin-loader', '--no-default-features', '--test', 'android_portable']).status === 0,
      'Android portable plugin probes',
    );
    report(
      cargoTest(['-p', 'tiktools-plugin-loader', '--no-default-features', '--lib', 'process::tests::']).status === 0,
      'process framing, environment, timeouts and cleanup',
    );
  } else {
    console.log('SKIPPED: Android portable plugin probes (native Android required)');
  }
  console.log('SKIPPED: SQLite stress in the default core build (executed with persistence in the full capability suite)');
  console.log('SKIPPED: desktop, frontend, real TikTok authentication/network, external native plugins and desktop native-addon fixtures');
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
