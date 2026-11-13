import { describe, expect, test } from 'bun:test';
import { chmod, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { checkAppImageSmokeInstallation, parseAppImageSmokeArgs } from './appimage-smoke.ts';

describe('AppImage smoke launch modes', () => {
  test('defaults to strict mounted classification unless extraction is explicitly selected', () => {
    expect(parseAppImageSmokeArgs(['release/TikTools.AppImage', '4.2.2'], 'linux')).toEqual({ imageArg: 'release/TikTools.AppImage', expectedVersion: '4.2.2', mode: 'mounted' });
    expect(parseAppImageSmokeArgs(['release/TikTools.AppImage', '4.2.2', '--extracted-mode'], 'linux').mode).toBe('extracted');
  });

  test('rejects unsupported platforms, incomplete arguments and arbitrary expected classifications', () => {
    for (const args of [[], ['app'], ['app', '4.2.2', 'unknown'], ['app', '4.2.2', '--expected-type=unknown'], ['app', '4.2.2', '--extracted-mode', 'extra']]) {
      expect(() => parseAppImageSmokeArgs(args, 'linux')).toThrow('usage on Linux');
    }
    expect(() => parseAppImageSmokeArgs(['app', '4.2.2'], 'win32')).toThrow('usage on Linux');
  });

  test('mounted mode requires appImage and rejects fail-closed or other installation types', () => {
    expect(() => checkAppImageSmokeInstallation('"appImage"', 'mounted')).not.toThrow();
    for (const kind of ['unknown', 'portable', 'nsis', 'deb', 'unsupported']) {
      expect(() => checkAppImageSmokeInstallation(JSON.stringify(kind), 'mounted')).toThrow('mounted installation detection failed');
    }
  });

  test('extracted mode accepts only the immutable detector fail-closed outcomes', () => {
    for (const kind of ['unknown', 'portable']) {
      expect(() => checkAppImageSmokeInstallation(JSON.stringify(kind), 'extracted')).not.toThrow();
    }
    for (const kind of ['appImage', 'nsis', 'deb', 'unsupported']) {
      expect(() => checkAppImageSmokeInstallation(JSON.stringify(kind), 'extracted')).toThrow('extracted installation detection failed');
    }
  });

  test('requires the actual JSON enum spelling and provides the observed output on failure', () => {
    expect(() => checkAppImageSmokeInstallation('appImage', 'mounted')).toThrow('invalid JSON');
    expect(() => checkAppImageSmokeInstallation('"appimage"', 'mounted')).toThrow('received "appimage"');
    for (const output of ['null', '{}', 'true', '0']) {
      expect(() => checkAppImageSmokeInstallation(output, 'extracted')).toThrow('installation detection failed');
    }
  });
});

test.skipIf(process.platform !== 'linux')('extracted fixture still requires actual frontend-ready after passing both CLI probes', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-smoke-gate-test-'));
  const fixture = join(root, 'fixture.AppImage');
  try {
    // This fixture deliberately does not launch a GUI or claim frontend-ready.
    await writeFile(fixture, '#!/bin/sh\ncase "${1:-}" in\n--version) echo 4.2.2 ;;\n--update-installation-type) printf \'"unknown"\\n\' ;;\n*) exit 0 ;;\nesac\n');
    await chmod(fixture, 0o755);
    const child = Bun.spawn([process.execPath, resolve(import.meta.dir, '../smoke-appimage.ts'), fixture, '4.2.2', '--extracted-mode'], { stdout: 'pipe', stderr: 'pipe' });
    const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    expect(out).toContain('--version (extracted): "4.2.2"');
    expect(out).toContain('--update-installation-type (extracted): "\\"unknown\\""');
    expect(code).not.toBe(0);
    expect(err).toContain('AppImage did not reach frontend-ready');
    expect(out).not.toContain('reached frontend-ready with an isolated profile');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test.skipIf(process.platform !== 'linux')('mounted fixture clears inherited extraction and retains strict classification', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-smoke-mount-test-'));
  const fixture = join(root, 'fixture.AppImage');
  try {
    await writeFile(fixture, '#!/bin/sh\ncase "${1:-}" in\n--version) echo 4.2.2 ;;\n--update-installation-type) if [ -n "${APPIMAGE_EXTRACT_AND_RUN:-}" ]; then echo inherited-extraction; else printf \'"unknown"\\n\'; fi ;;\n*) exit 0 ;;\nesac\n');
    await chmod(fixture, 0o755);
    const child = Bun.spawn([process.execPath, resolve(import.meta.dir, '../smoke-appimage.ts'), fixture, '4.2.2'], { env: { ...process.env, APPIMAGE_EXTRACT_AND_RUN: '1' }, stdout: 'pipe', stderr: 'pipe' });
    const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    expect(code).not.toBe(0);
    expect(err).toContain('mounted installation detection failed: received "unknown"; expected appImage');
    expect(out).not.toContain('inherited-extraction');
  } finally { await rm(root, { recursive: true, force: true }); }
});
