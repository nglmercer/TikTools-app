import assert from 'node:assert/strict';
import test from 'node:test';

import { resolveCargoEnvironment } from './cargo-with-linker.mjs';

const X64_LINKER = 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER';
const ARM64_LINKER = 'CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER';

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

test('does not alter linker configuration outside Windows', () => {
  let detectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'linux',
    hasCommand: () => {
      detectionAttempted = true;
      return true;
    },
  });

  assert.equal(detectionAttempted, false);
  assert.equal(Object.hasOwn(resolved.environment, X64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, ARM64_LINKER), false);
});
