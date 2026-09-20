import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveCargoEnvironment } from './cargo-with-linker.mjs';

const X64_LINKER = 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER';
const ARM64_LINKER = 'CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER';
const LINUX_LINKER = 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER';

test('uses lld-link for unconfigured Windows MSVC targets when available', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'win32',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[X64_LINKER], 'lld-link');
  assert.equal(resolved.environment[ARM64_LINKER], 'lld-link');
  assert.equal(resolved.linker, 'lld-link');
});

test('leaves Windows MSVC targets unset when lld-link is unavailable', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'win32',
    hasCommand: () => false,
  });

  assert.equal(Object.hasOwn(resolved.environment, X64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, ARM64_LINKER), false);
  assert.equal(resolved.linker, 'default');
});

test('preserves user linker overrides ahead of detected lld-link', () => {
  const resolved = resolveCargoEnvironment({
    environment: { [X64_LINKER]: 'custom-linker' },
    platform: 'win32',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[X64_LINKER], 'custom-linker');
  assert.equal(resolved.environment[ARM64_LINKER], 'lld-link');
});

test('does not run detection when users configured both Windows targets', () => {
  let detectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { [X64_LINKER]: 'x64-linker', [ARM64_LINKER]: 'arm64-linker' },
    platform: 'win32',
    hasCommand: () => {
      detectionAttempted = true;
      return true;
    },
  });

  assert.equal(detectionAttempted, false);
  assert.equal(resolved.environment[X64_LINKER], 'x64-linker');
  assert.equal(resolved.environment[ARM64_LINKER], 'arm64-linker');
});

test('does not alter linker configuration outside Windows and Linux', () => {
  let detectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'darwin',
    hasCommand: () => {
      detectionAttempted = true;
      return true;
    },
  });

  assert.equal(detectionAttempted, false);
  assert.equal(Object.hasOwn(resolved.environment, X64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, ARM64_LINKER), false);
});

test('uses mold on Linux when available', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'linux',
    hasCommand: (command) => command === 'mold',
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C link-arg=-fuse-ld=mold');
  assert.equal(resolved.linker, 'mold');
});

test('leaves Linux linker flags unchanged when mold is unavailable', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', RUSTFLAGS: '-C debuginfo=1' },
    platform: 'linux',
    hasCommand: () => false,
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C debuginfo=1');
  assert.equal(resolved.linker, 'default');
});

test('preserves an existing Linux linker override ahead of mold', () => {
  const resolved = resolveCargoEnvironment({
    environment: { [LINUX_LINKER]: 'clang' },
    platform: 'linux',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[LINUX_LINKER], 'clang');
  assert.equal(Object.hasOwn(resolved.environment, 'RUSTFLAGS'), false);
  assert.equal(resolved.linker, 'unchanged');
});

test('appends mold to encoded Rust flags when available', () => {
  const resolved = resolveCargoEnvironment({
    environment: {
      CARGO_ENCODED_RUSTFLAGS: '-C\u001fdebuginfo=1',
    },
    platform: 'linux',
    hasCommand: () => true,
  });

  assert.equal(
    resolved.environment.CARGO_ENCODED_RUSTFLAGS,
    '-C\u001fdebuginfo=1\u001f-C\u001flink-arg=-fuse-ld=mold',
  );
  assert.equal(Object.hasOwn(resolved.environment, 'RUSTFLAGS'), false);
});
