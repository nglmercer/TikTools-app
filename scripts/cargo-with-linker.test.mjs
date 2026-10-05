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
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'lld-link');
  assert.equal(resolved.kache, 'enabled');
});

test('leaves Windows MSVC targets unset when lld-link is unavailable', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'win32',
    hasCommand: () => false,
  });

  assert.equal(Object.hasOwn(resolved.environment, X64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, ARM64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, 'RUSTC_WRAPPER'), false);
  assert.equal(resolved.linker, 'default');
  assert.equal(resolved.kache, 'unavailable');
});

test('preserves user linker overrides ahead of detected lld-link', () => {
  const resolved = resolveCargoEnvironment({
    environment: { [X64_LINKER]: 'custom-linker' },
    platform: 'win32',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[X64_LINKER], 'custom-linker');
  assert.equal(resolved.environment[ARM64_LINKER], 'lld-link');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
});

test('does not run linker detection when users configured both Windows targets', () => {
  let linkerDetectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { [X64_LINKER]: 'x64-linker', [ARM64_LINKER]: 'arm64-linker' },
    platform: 'win32',
    hasCommand: (command) => {
      if (command === 'lld-link.exe') {
        linkerDetectionAttempted = true;
      }
      return true;
    },
  });

  assert.equal(linkerDetectionAttempted, false);
  assert.equal(resolved.environment[X64_LINKER], 'x64-linker');
  assert.equal(resolved.environment[ARM64_LINKER], 'arm64-linker');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
});

test('does not alter linker configuration outside Windows and Linux', () => {
  let linkerDetectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'darwin',
    hasCommand: (command) => {
      if (command === 'mold' || command === 'lld-link.exe') {
        linkerDetectionAttempted = true;
      }
      return true;
    },
  });

  assert.equal(linkerDetectionAttempted, false);
  assert.equal(Object.hasOwn(resolved.environment, X64_LINKER), false);
  assert.equal(Object.hasOwn(resolved.environment, ARM64_LINKER), false);
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'unchanged');
});

test('uses mold on Linux when available', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'linux',
    hasCommand: (command) => command === 'mold',
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C link-arg=-fuse-ld=mold');
  assert.equal(Object.hasOwn(resolved.environment, 'RUSTC_WRAPPER'), false);
  assert.equal(resolved.linker, 'mold');
  assert.equal(resolved.kache, 'unavailable');
});

test('leaves Linux linker flags unchanged when mold is unavailable', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', RUSTFLAGS: '-C debuginfo=1' },
    platform: 'linux',
    hasCommand: () => false,
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C debuginfo=1');
  assert.equal(resolved.linker, 'default');
  assert.equal(resolved.kache, 'unavailable');
});

test('preserves an existing Linux linker override ahead of mold', () => {
  const resolved = resolveCargoEnvironment({
    environment: { [LINUX_LINKER]: 'clang' },
    platform: 'linux',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[LINUX_LINKER], 'clang');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
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
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
});

test('uses kache by default when installed', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache' || command === 'mold',
  });

  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.kache, 'enabled');
});

test('skips kache silently when it is not installed', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test' },
    platform: 'linux',
    hasCommand: (command) => command === 'mold',
  });

  assert.equal(Object.hasOwn(resolved.environment, 'RUSTC_WRAPPER'), false);
  assert.equal(resolved.kache, 'unavailable');
});

test('opts out of kache with TIKTOOLS_KACHE=0', () => {
  let kacheDetectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '0' },
    platform: 'linux',
    hasCommand: (command) => {
      if (command === 'kache') {
        kacheDetectionAttempted = true;
      }
      return true;
    },
  });

  assert.equal(kacheDetectionAttempted, false);
  assert.equal(Object.hasOwn(resolved.environment, 'RUSTC_WRAPPER'), false);
  assert.equal(resolved.kache, 'disabled');
});

test('treats an empty TIKTOOLS_KACHE as the default', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache' || command === 'mold',
  });

  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.kache, 'enabled');
});

test('sets kache as the rustc wrapper when requested and available', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '1' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache' || command === 'mold',
  });

  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.kache, 'enabled');
});

test('reports kache as missing when requested but unavailable', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '1' },
    platform: 'linux',
    hasCommand: () => false,
  });

  assert.equal(Object.hasOwn(resolved.environment, 'RUSTC_WRAPPER'), false);
  assert.equal(resolved.kache, 'missing');
});

test('preserves an existing rustc wrapper ahead of kache', () => {
  let kacheDetectionAttempted = false;
  const resolved = resolveCargoEnvironment({
    environment: { RUSTC_WRAPPER: 'sccache', TIKTOOLS_KACHE: '1' },
    platform: 'linux',
    hasCommand: (command) => {
      if (command === 'kache') {
        kacheDetectionAttempted = true;
      }
      return command === 'mold';
    },
  });

  assert.equal(kacheDetectionAttempted, false);
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'sccache');
  assert.equal(resolved.kache, 'preserved');
});

test('combines kache with mold on Linux', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '1' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache' || command === 'mold',
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C link-arg=-fuse-ld=mold');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'mold');
  assert.equal(resolved.kache, 'enabled');
});

test('combines kache with lld-link on Windows', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '1' },
    platform: 'win32',
    hasCommand: () => true,
  });

  assert.equal(resolved.environment[X64_LINKER], 'lld-link');
  assert.equal(resolved.environment[ARM64_LINKER], 'lld-link');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'lld-link');
  assert.equal(resolved.kache, 'enabled');
});

test('keeps custom Rust flags alongside kache', () => {
  const resolved = resolveCargoEnvironment({
    environment: {
      PATH: 'test',
      TIKTOOLS_KACHE: '1',
      RUSTFLAGS: '-C debuginfo=1',
    },
    platform: 'linux',
    hasCommand: (command) => command === 'kache' || command === 'mold',
  });

  assert.equal(resolved.environment.RUSTFLAGS, '-C debuginfo=1 -C link-arg=-fuse-ld=mold');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.kache, 'enabled');
});

test('keeps a configured Linux linker alongside kache', () => {
  const resolved = resolveCargoEnvironment({
    environment: { [LINUX_LINKER]: 'clang', TIKTOOLS_KACHE: '1' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache',
  });

  assert.equal(resolved.environment[LINUX_LINKER], 'clang');
  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'unchanged');
  assert.equal(resolved.kache, 'enabled');
});

test('applies kache on platforms without linker detection', () => {
  const resolved = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: '1' },
    platform: 'darwin',
    hasCommand: (command) => command === 'kache',
  });

  assert.equal(resolved.environment.RUSTC_WRAPPER, 'kache');
  assert.equal(resolved.linker, 'unchanged');
  assert.equal(resolved.kache, 'enabled');
});

test('accepts alternative kache values', () => {
  const required = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: 'YES' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache',
  });
  assert.equal(required.kache, 'enabled');
  assert.equal(required.environment.RUSTC_WRAPPER, 'kache');

  const alsoRequired = resolveCargoEnvironment({
    environment: { PATH: 'test', TIKTOOLS_KACHE: 'true' },
    platform: 'linux',
    hasCommand: (command) => command === 'kache',
  });
  assert.equal(alsoRequired.kache, 'enabled');

  for (const value of ['0', 'false', 'no']) {
    const optedOut = resolveCargoEnvironment({
      environment: { PATH: 'test', TIKTOOLS_KACHE: value },
      platform: 'linux',
      hasCommand: (command) => {
        if (command === 'kache') {
          throw new Error('kache detection must not run when opted out');
        }
        return true;
      },
    });
    assert.equal(optedOut.kache, 'disabled');
    assert.equal(Object.hasOwn(optedOut.environment, 'RUSTC_WRAPPER'), false);
  }
});
