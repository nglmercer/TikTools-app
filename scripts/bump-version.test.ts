import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, test } from 'bun:test';
import {
  assertVersionUpgrade,
  normalizeVersion,
  readPackageName,
  readWorkspaceMembers,
  readWorkspaceVersion,
  setCargoLockVersions,
  setPackageJsonVersion,
  setPathDependencyVersions,
  setWorkspaceVersion,
  updateStandaloneLockfiles,
  usesWorkspaceVersion,
} from './bump-version.ts';

describe('normalizeVersion', () => {
  test('compares prereleases with SemVer precedence and rejects downgrades', () => {
    expect(() => assertVersionUpgrade('1.0.0-beta.1', '1.0.0')).not.toThrow();
    expect(() => assertVersionUpgrade('1.0.0', '1.0.0-beta.1')).toThrow();
    expect(() => assertVersionUpgrade('1.0.0+one', '1.0.0+two')).toThrow();
  });
  test('accepts plain semver', () => {
    expect(normalizeVersion('0.3.1')).toBe('0.3.1');
  });

  test('strips a leading v', () => {
    expect(normalizeVersion('v0.3.1')).toBe('0.3.1');
  });

  test('accepts prerelease versions', () => {
    expect(normalizeVersion('v1.0.0-beta.1')).toBe('1.0.0-beta.1');
  });

  test('rejects non-semver input', () => {
    expect(() => normalizeVersion('0.3')).toThrow();
    expect(() => normalizeVersion('release')).toThrow();
    expect(() => normalizeVersion('')).toThrow();
  });
});

describe('cargo manifest helpers', () => {
  test('updates only matching workspace path dependency pins', () => {
    const input = 'tiktools-api = { path = "../api", version = "0.3.3" }\nother = { path = "../other", version = "0.3.3" }\ntiktools-api-new = { path = "../api", version = "0.3.30" }\n';
    const updated = setPathDependencyVersions(input, ['tiktools-api', 'tiktools-api-new'], '0.3.3', '4.0.0');
    expect(updated).toContain('tiktools-api = { path = "../api", version = "4.0.0" }');
    expect(updated).toContain('other = { path = "../other", version = "0.3.3" }');
    expect(updated).toContain('version = "0.3.30"');
  });
  const cargoToml = `[workspace]
members = [
    "crates/foo",
    "crates/bar",
]
resolver = "2"

[workspace.package]
version = "0.2.0"
edition = "2021"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
`;

  test('readWorkspaceVersion reads the workspace version', () => {
    expect(readWorkspaceVersion(cargoToml)).toBe('0.2.0');
  });

  test('readWorkspaceMembers lists member directories', () => {
    expect(readWorkspaceMembers(cargoToml)).toEqual(['crates/foo', 'crates/bar']);
  });

  test('readPackageName reads the member crate name', () => {
    expect(readPackageName('[package]\nname = "tiktools-foo"\nversion.workspace = true\n')).toBe(
      'tiktools-foo',
    );
  });

  test('setWorkspaceVersion only touches the workspace version', () => {
    const updated = setWorkspaceVersion(cargoToml, '0.3.1');
    expect(updated).toContain('[workspace.package]\nversion = "0.3.1"');
    expect(updated).toContain('serde = { version = "1.0", features = ["derive"] }');
    expect(readWorkspaceVersion(updated)).toBe('0.3.1');
  });

  test('setWorkspaceVersion throws when the version is missing', () => {
    expect(() => setWorkspaceVersion('[workspace.package]\nedition = "2021"\n', '0.3.1')).toThrow();
  });

  test('usesWorkspaceVersion detects inherited vs independent versions', () => {
    expect(usesWorkspaceVersion('[package]\nname = "a"\nversion.workspace = true\n')).toBe(true);
    expect(usesWorkspaceVersion('[package]\nname = "b"\nversion = "1.0.0"\n')).toBe(false);
  });
});

describe('setPackageJsonVersion', () => {
  test('updates the version key preserving the rest', () => {
    const updated = setPackageJsonVersion(
      JSON.stringify({ name: 'app', version: '0.2.0', private: true }, null, 2),
      '0.3.1',
    );
    expect(JSON.parse(updated)).toEqual({ name: 'app', version: '0.3.1', private: true });
  });
});

describe('setCargoLockVersions', () => {
  const lock = `[[package]]
name = "tiktools-desktop"
version = "0.2.0"
dependencies = [
 "gtk",
]

[[package]]
name = "third-party"
version = "0.2.0"

[[package]]
name = "tiktools-core"
version = "0.2.0"
`;

  test('bumps only the listed workspace packages', () => {
    const result = setCargoLockVersions(lock, ['tiktools-desktop', 'tiktools-core'], '0.2.0', '0.3.1');
    expect(result.updated).toEqual(['tiktools-desktop', 'tiktools-core']);
    expect(result.missing).toEqual([]);
    expect(result.text).toContain('name = "tiktools-desktop"\nversion = "0.3.1"');
    expect(result.text).toContain('name = "third-party"\nversion = "0.2.0"');
  });

  test('reports packages without a matching entry', () => {
    const result = setCargoLockVersions(lock, ['tiktools-desktop', 'tiktools-missing'], '0.2.0', '0.3.1');
    expect(result.updated).toEqual(['tiktools-desktop']);
    expect(result.missing).toEqual(['tiktools-missing']);
  });

  test('does not touch entries already at another version', () => {
    const result = setCargoLockVersions(lock, ['tiktools-desktop'], '9.9.9', '0.3.1');
    expect(result.text).toBe(lock);
    expect(result.missing).toEqual(['tiktools-desktop']);
  });
});

describe('updateStandaloneLockfiles', () => {
  test('refreshes standalone locks and skips locks without workspace pins', async () => {
    const root = await mkdtemp(join(tmpdir(), 'bump-version-test-'));
    try {
      const stale = join(root, 'crates/alpha/tests/fixtures/stale');
      const clean = join(root, 'crates/alpha/tests/fixtures/clean');
      const fixture = join(root, 'crates/loader/tests/fixtures/probe');
      await mkdir(stale, { recursive: true });
      await mkdir(clean, { recursive: true });
      await mkdir(fixture, { recursive: true });
      await writeFile(
        join(stale, 'Cargo.lock'),
        '[[package]]\nname = "tiktools-plugin-api"\nversion = "0.2.0"\n\n[[package]]\nname = "other"\nversion = "0.2.0"\n',
      );
      await writeFile(join(clean, 'Cargo.lock'), '[[package]]\nname = "other"\nversion = "0.2.0"\n');
      await writeFile(
        join(fixture, 'Cargo.lock'),
        '[[package]]\nname = "tiktools-plugin-sdk"\nversion = "0.2.0"\n',
      );

      const results = await updateStandaloneLockfiles(
        root,
        ['tiktools-plugin-api', 'tiktools-plugin-sdk'],
        '0.2.0',
        '0.3.2',
      );

      expect(results).toEqual([
        { path: 'crates/alpha/tests/fixtures/stale/Cargo.lock', updated: ['tiktools-plugin-api'] },
        {
          path: 'crates/loader/tests/fixtures/probe/Cargo.lock',
          updated: ['tiktools-plugin-sdk'],
        },
      ]);
      const staleText = await readFile(join(stale, 'Cargo.lock'), 'utf8');
      expect(staleText).toContain('name = "tiktools-plugin-api"\nversion = "0.3.2"');
      expect(staleText).toContain('name = "other"\nversion = "0.2.0"');
      await expect(readFile(join(clean, 'Cargo.lock'), 'utf8')).resolves.toContain(
        'version = "0.2.0"',
      );
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
