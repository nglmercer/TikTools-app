import { describe, expect, test } from 'bun:test';
import {
  artifactFileName,
  detectHostTarget,
  pluginTargetFromRustTarget,
  resolveRustTarget,
} from './plugin-targets.ts';

describe('plugin target mapping', () => {
  const cases: Array<[string, string]> = [
    ['x86_64-pc-windows-msvc', 'win32-x64-msvc'],
    ['aarch64-pc-windows-msvc', 'win32-arm64-msvc'],
    ['x86_64-unknown-linux-gnu', 'linux-x64-gnu'],
    ['aarch64-unknown-linux-gnu', 'linux-arm64-gnu'],
    ['x86_64-apple-darwin', 'darwin-x64-darwin'],
    ['aarch64-apple-darwin', 'darwin-arm64-darwin'],
  ];

  for (const [rustTarget, pluginTarget] of cases) {
    test(`${rustTarget} -> ${pluginTarget}`, () => {
      expect(pluginTargetFromRustTarget(rustTarget)).toBe(pluginTarget);
      expect(resolveRustTarget(rustTarget).pluginTarget).toBe(pluginTarget);
    });
  }

  test('windows targets use .exe, others do not', () => {
    expect(resolveRustTarget('x86_64-pc-windows-msvc').executableExtension).toBe('.exe');
    expect(resolveRustTarget('aarch64-pc-windows-msvc').executableExtension).toBe('.exe');
    expect(resolveRustTarget('x86_64-unknown-linux-gnu').executableExtension).toBe('');
    expect(resolveRustTarget('aarch64-apple-darwin').executableExtension).toBe('');
  });

  test('unsupported targets produce clear errors', () => {
    for (const unsupported of [
      'i686-pc-windows-msvc',
      'wasm32-unknown-unknown',
      'x86_64-unknown-linux-musl',
    ]) {
      expect(() => resolveRustTarget(unsupported)).toThrow(/Unsupported Rust target/);
    }
  });
});

describe('host target detection', () => {
  test('maps known platform/arch combinations', () => {
    expect(detectHostTarget('win32', 'x64').rustTarget).toBe('x86_64-pc-windows-msvc');
    expect(detectHostTarget('win32', 'arm64').rustTarget).toBe('aarch64-pc-windows-msvc');
    expect(detectHostTarget('linux', 'x64').rustTarget).toBe('x86_64-unknown-linux-gnu');
    expect(detectHostTarget('linux', 'arm64').rustTarget).toBe('aarch64-unknown-linux-gnu');
    expect(detectHostTarget('darwin', 'x64').rustTarget).toBe('x86_64-apple-darwin');
    expect(detectHostTarget('darwin', 'arm64').rustTarget).toBe('aarch64-apple-darwin');
  });

  test('rejects unsupported host combinations', () => {
    expect(() => detectHostTarget('freebsd' as never, 'x64')).toThrow(
      /Unsupported host combination/,
    );
  });
});

describe('artifact filenames', () => {
  test('targeted artifacts include id, version, and target', () => {
    expect(artifactFileName('audio-process', '1.0.0', 'win32-x64-msvc')).toBe(
      'audio-process-1.0.0-win32-x64-msvc.plugin',
    );
  });

  test('target-independent artifacts omit the target', () => {
    expect(artifactFileName('my-wasm-plugin', '1.0.0', null)).toBe('my-wasm-plugin-1.0.0.plugin');
  });
});
