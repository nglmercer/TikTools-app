import assert from 'node:assert/strict';
import test from 'node:test';

import {
  mergeEnvironment,
  parseEnvironmentDump,
  resolveCargoEnvironment,
  resolveMsvcEnvironment,
  resolveWindowsPerlEnvironment,
} from './cargo-with-linker.mjs';

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

test('loads the MSVC developer environment when INCLUDE is missing on Windows', () => {
  let capturedArchitecture;
  const resolved = resolveMsvcEnvironment({
    environment: { Path: 'C:\\bin' },
    platform: 'win32',
    architecture: 'x64',
    findDevCmd: () => 'C:\\VS\\VsDevCmd.bat',
    captureEnvironment: (_devCmd, architecture) => {
      capturedArchitecture = architecture;
      return 'INCLUDE=C:\\include\r\nPath=C:\\msvc;C:\\bin\r\n';
    },
  });

  assert.equal(capturedArchitecture, 'x64');
  assert.equal(resolved.status, 'loaded');
  assert.equal(resolved.environment.INCLUDE, 'C:\\include');
});

test('keeps an existing Developer prompt environment', () => {
  const resolved = resolveMsvcEnvironment({
    environment: { INCLUDE: 'C:\\include' },
    platform: 'win32',
    findDevCmd: () => {
      throw new Error('Visual Studio detection must not run inside a Developer prompt');
    },
  });

  assert.equal(resolved.status, 'preserved');
});

test('skips MSVC environment loading outside Windows', () => {
  const resolved = resolveMsvcEnvironment({ environment: {}, platform: 'linux' });

  assert.equal(resolved.status, 'unchanged');
});

test('reports MSVC environment as unavailable without Visual Studio', () => {
  const resolved = resolveMsvcEnvironment({
    environment: {},
    platform: 'win32',
    architecture: 'x64',
    findDevCmd: () => undefined,
  });

  assert.equal(resolved.status, 'unavailable');
});

test('reports a failed MSVC environment capture without INCLUDE', () => {
  const resolved = resolveMsvcEnvironment({
    environment: {},
    platform: 'win32',
    architecture: 'x64',
    findDevCmd: () => 'C:\\VS\\VsDevCmd.bat',
    captureEnvironment: () => 'Path=C:\\bin',
  });

  assert.equal(resolved.status, 'failed');
});

test('parses environment dumps and merges them case-insensitively', () => {
  const loaded = parseEnvironmentDump('Path=C:\\msvc\r\nINCLUDE=C:\\a=b\r\n\r\n');
  const merged = mergeEnvironment({ PATH: 'C:\\old', KEEP: '1' }, loaded);

  assert.deepEqual(merged, { KEEP: '1', Path: 'C:\\msvc', INCLUDE: 'C:\\a=b' });
});

test('uses native Windows Perl already on PATH', () => {
  const environment = { Path: 'C:\\native-perl\\bin' };
  const resolved = resolveWindowsPerlEnvironment({
    environment,
    platform: 'win32',
    probe: (command, childEnvironment) => {
      assert.equal(command, 'perl');
      assert.deepEqual(childEnvironment, environment);
      return true;
    },
    fileExists: () => { throw new Error('installed Perl needs no fallback detection'); },
  });

  assert.equal(resolved.status, 'available');
  assert.deepEqual(resolved.environment, environment);
});

test('configures Strawberry Perl outside PATH without adding its GCC tools', () => {
  const environment = { Path: 'C:\\msvc;C:\\bin', INCLUDE: 'C:\\include' };
  const command = 'C:\\Strawberry\\perl\\bin\\perl.exe';
  const resolved = resolveWindowsPerlEnvironment({
    environment,
    platform: 'win32',
    fileExists: (path) => path === command,
    probe: (path, childEnvironment) => {
      assert.deepEqual(childEnvironment, environment);
      return path === command;
    },
  });

  assert.equal(resolved.status, 'configured');
  assert.equal(resolved.command, command);
  assert.deepEqual(resolved.environment, { ...environment, OPENSSL_SRC_PERL: command });
  assert.equal(Object.hasOwn(environment, 'OPENSSL_SRC_PERL'), false);
});

test('finds native Perl on another system drive and in paths with spaces', () => {
  for (const [environment, command] of [
    [{ systemdrive: 'D:' }, 'D:\\Strawberry\\perl\\bin\\perl.exe'],
    [{ PROGRAMFILES: 'D:\\Program Files' }, 'D:\\Program Files\\Strawberry\\perl\\bin\\perl.exe'],
    [{ localappdata: 'D:\\Users\\A B\\AppData\\Local' }, 'D:\\Users\\A B\\AppData\\Local\\Programs\\Strawberry\\perl\\bin\\perl.exe'],
  ]) {
    const resolved = resolveWindowsPerlEnvironment({
      environment,
      platform: 'win32',
      fileExists: (path) => path === command,
      probe: (path) => path === command,
    });

    assert.equal(resolved.status, 'configured');
    assert.equal(resolved.environment.OPENSSL_SRC_PERL, command);
  }
});

test('preserves explicit OpenSSL and Perl overrides including portable installs', () => {
  for (const environment of [
    { OPENSSL_SRC_PERL: 'D:\\Portable Perl\\perl.exe', PERL: 'other-perl' },
    { PERL: 'custom-perl' },
    { openssl_src_perl: 'D:\\perl.exe' },
    { OPENSSL_SRC_PERL: '' },
  ]) {
    const resolved = resolveWindowsPerlEnvironment({
      environment,
      platform: 'win32',
      probe: () => { throw new Error('explicit Perl configuration must win'); },
    });

    assert.equal(resolved.status, 'preserved');
    assert.deepEqual(resolved.environment, environment);
  }
});

test('reports missing or incompatible Perl without requiring it for unrelated Cargo commands', () => {
  for (const fileExists of [() => false, () => true]) {
    const resolved = resolveWindowsPerlEnvironment({
      environment: { Path: 'C:\\Git\\usr\\bin' },
      platform: 'win32',
      fileExists,
      probe: () => false,
    });

    assert.equal(resolved.status, 'unavailable');
    assert.deepEqual(resolved.environment, { Path: 'C:\\Git\\usr\\bin' });
  }
});

test('does not probe Windows Perl on other platforms', () => {
  for (const platform of ['linux', 'darwin']) {
    const environment = { PATH: '/usr/bin' };
    const resolved = resolveWindowsPerlEnvironment({
      environment,
      platform,
      probe: () => { throw new Error('Windows Perl detection must not run'); },
    });

    assert.equal(resolved.status, 'unchanged');
    assert.deepEqual(resolved.environment, environment);
  }
});

test('detects optional tools using the prepared MSVC environment passed to Cargo', () => {
  const environment = { Path: 'C:\\VS\\bin', INCLUDE: 'C:\\include', OPENSSL_SRC_PERL: 'C:\\perl.exe' };
  const calls = [];
  const resolved = resolveCargoEnvironment({
    environment,
    platform: 'win32',
    hasCommand: (command, platform, childEnvironment) => {
      assert.equal(platform, 'win32');
      assert.deepEqual(childEnvironment, { ...environment, ...(command === 'lld-link.exe' ? { RUSTC_WRAPPER: 'kache' } : {}) });
      calls.push(command);
      return true;
    },
  });

  assert.deepEqual(calls, ['kache', 'lld-link.exe']);
  assert.equal(resolved.environment.OPENSSL_SRC_PERL, environment.OPENSSL_SRC_PERL);
  assert.equal(resolved.linker, 'lld-link');
});

test('discovers native Windows Perl with the real process probe and no Perl on PATH', {
  skip: process.platform !== 'win32' || process.env.TIKTOOLS_TEST_WINDOWS_PERL !== '1',
}, () => {
  const environment = { ...process.env };
  for (const name of Object.keys(environment)) {
    if (['path', 'perl', 'openssl_src_perl'].includes(name.toLowerCase())) delete environment[name];
  }
  environment.Path = `${process.env.SystemRoot || 'C:\\Windows'}\\System32`;
  const resolved = resolveWindowsPerlEnvironment({ environment });

  assert.equal(resolved.status, 'configured');
  assert.match(resolved.environment.OPENSSL_SRC_PERL, /perl\.exe$/i);
  assert.equal(resolved.environment.Path, environment.Path);
});
