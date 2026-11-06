import { describe, expect, test } from 'bun:test';
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { readWorkspaceVersion } from './bump-version.ts';
import { validateVersions } from './check-version.ts';

describe('release version validation', () => {
  test('rejects the historical v0.5.0 tag with 0.3.3 manifests', () => {
    expect(() => validateVersions('0.3.3', '0.3.3', 'v0.5.0', '0.3.3')).toThrow();
  });
  test('accepts matching prerelease versions', () => {
    expect(validateVersions('1.0.0-beta.1', '1.0.0-beta.1', 'v1.0.0-beta.1', '1.0.0-beta.1')).toBe('1.0.0-beta.1');
  });
  test('rejects malformed tags and mismatched binary', () => {
    for (const tag of ['1.0.0', 'v01.0.0', 'v1.0.0-beta..1', 'v1.0.0-01', 'v1.0.0/evil']) {
      expect(() => validateVersions('1.0.0', '1.0.0', tag)).toThrow();
    }
    expect(() => validateVersions('1.0.0', '1.0.0', 'v1.0.0', '0.9.0')).toThrow();
  });
  test.skipIf(process.platform === 'win32')('CLI rejects a binary reporting a different version', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'tiktools-version-test-'));
    try {
      const binary = join(dir, 'desktop');
      await writeFile(binary, '#!/bin/sh\nprintf "0.3.3\\n"\n');
      await chmod(binary, 0o755);
      const root = resolve(import.meta.dir, '..');
      const version = readWorkspaceVersion(await readFile(join(root, 'Cargo.toml'), 'utf8'));
      const child = Bun.spawn([process.execPath, 'scripts/check-version.ts', '--binary'], {
        cwd: root,
        env: { ...process.env, RELEASE_TAG: `v${version}`, RELEASE_BINARY: binary },
        stdout: 'pipe', stderr: 'pipe',
      });
      const [stderr, code] = await Promise.all([new Response(child.stderr).text(), child.exited]);
      expect(code).not.toBe(0);
      expect(stderr).toContain('desktop binary version');
    } finally { await rm(dir, { recursive: true, force: true }); }
  });

});
