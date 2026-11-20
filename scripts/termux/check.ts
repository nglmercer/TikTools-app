#!/usr/bin/env node
// Termux toolchain check, then `cargo check -p tiktools-cli`.
// Replaces scripts/termux/check.sh. The former embedded Python version probe is now JavaScript.
import { commandPath, cargoJobs, hostTriple, run, exitWith } from './common.ts';

function compareVersions(a: number[], b: number[]): number {
  for (let index = 0; index < 3; index += 1) {
    if (a[index]! !== b[index]!) return a[index]! - b[index]!;
  }
  return 0;
}

const REQUIRED = ['rustc', 'cargo', 'clang', 'cmake', 'pkg-config', 'protoc'];
for (const tool of REQUIRED) {
  if (!commandPath(tool)) {
    console.error(`Missing ${tool}; install the Termux rust, clang, cmake, pkg-config and protobuf packages`);
    process.exit(1);
  }
}

const show = (command: string, args: string[]) => {
  const result = run(command, args, { inherit: true });
  if (result.status !== 0) exitWith(result);
};
show('rustc', ['-vV']);
show('cargo', ['--version']);
show('clang', ['--version']);

const rustVersion = run('rustc', ['--version']).stdout ?? '';
const match = /(\d+)\.(\d+)\.(\d+)/.exec(rustVersion);
const numbers = match ? match.slice(1).map(Number) : [];
const minimum = [1, 95, 0];
// Lexicographic comparison of major, minor and patch.
const atLeast195 = numbers.length === 3 && compareVersions(numbers, minimum) >= 0;
if (!atLeast195) {
  console.error('TikTools requires Rust >= 1.95; update the Termux rust package');
  process.exit(1);
}

const host = hostTriple();
if (host.includes('android')) {
  console.log(`Native Android compiler: ${host} (${run('uname', ['-m']).stdout?.trim()})`);
} else {
  console.error(`Notice: ${host} is not native Android; this cannot validate Termux acceptance`);
}

show('df', ['-h', '.']);
const check = run('cargo', ['check', '-p', 'tiktools-cli', '--locked'], {
  inherit: true,
  env: { ...process.env, CARGO_BUILD_JOBS: cargoJobs() },
});
exitWith(check);
