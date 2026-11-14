import { describe, expect, test } from 'bun:test';
import { chmod, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { readFile } from 'node:fs/promises';
import { checkAppImageCli, checkAppImageSmokeInstallation, parseAppImageSmokeArgs } from './appimage-smoke.ts';

describe('AppImage smoke launch modes', () => {
  test('defaults to strict mounted classification unless extraction is explicitly selected', () => {
    expect(parseAppImageSmokeArgs(['release/TikTools.AppImage', '4.2.2'], 'linux')).toEqual({ imageArg: 'release/TikTools.AppImage', expectedVersion: '4.2.2', mode: 'mounted', requireCli: false });
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

test.skipIf(process.platform !== 'linux')('frontend-ready smoke exits when a descendant retains output and ignores wrapper signals', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-smoke-descendant-'));
  const fixture = join(root, 'fixture.AppImage');
  const pidFile = join(root, 'descendant.pid');
  let descendant = 0;
  try {
    await writeFile(fixture, `#!/bin/sh
case "\${1:-}" in
--version) echo 4.2.2 ;;
--update-installation-type) printf '"unknown"\\n' ;;
*)
  mkdir -p "$TIKTOOLS_HOME/logs"
  echo 'WebView frontend-ready' > "$TIKTOOLS_HOME/logs/tiktools.log"
  sh -c 'trap "" INT TERM; while :; do sleep 1; done' &
  echo $! > "$SMOKE_DESCENDANT_PID_FILE"
  printf '%s ' "$XDG_RUNTIME_DIR" > "$SMOKE_DESCENDANT_PID_FILE.runtime"
  stat -c %a "$XDG_RUNTIME_DIR" >> "$SMOKE_DESCENDANT_PID_FILE.runtime"
  trap 'exit 0' INT TERM
  wait
  ;;
esac
`);
    await chmod(fixture, 0o755);
    const child = Bun.spawn([process.execPath, resolve(import.meta.dir, '../smoke-appimage.ts'), fixture, '4.2.2', '--extracted-mode'], { env: { ...process.env, SMOKE_DESCENDANT_PID_FILE: pidFile, XDG_RUNTIME_DIR: '/tmp/existing-real-desktop-runtime' }, stdout: 'pipe', stderr: 'pipe' });
    const killer = setTimeout(() => child.kill('SIGKILL'), 8000);
    const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]).finally(() => clearTimeout(killer));
    descendant = Number(await readFile(pidFile, 'utf8'));
    expect(await readFile(`${pidFile}.runtime`, 'utf8')).toMatch(/^\/tmp\/tiktools-appimage-smoke-[^/]+\/runtime 700\n$/);
    expect(out).toContain('reached frontend-ready with an isolated profile');
    expect(err).not.toContain('did not reach frontend-ready');
    expect(code).toBe(0);
    // An orphan may briefly be a zombie awaiting its OS reaper; it must not be running.
    const status = await readFile(`/proc/${descendant}/status`, 'utf8').catch(() => 'State:\tZ');
    expect(status).toMatch(/^State:\s+Z/m);
  } finally {
    if (descendant) { try { process.kill(descendant, 'SIGKILL'); } catch { /* Already reaped. */ } }
    await rm(root, { recursive: true, force: true });
  }
}, 10_000);

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

test('new AppImages must expose the real companion CLI when explicitly required', () => {
  expect(parseAppImageSmokeArgs(['app', '4.2.2', '--require-cli', '--extracted-mode'], 'linux').requireCli).toBe(true);
  expect(() => parseAppImageSmokeArgs(['app', '4.2.2', '--require-cli', '--require-cli'], 'linux')).toThrow('usage');
  expect(() => checkAppImageCli(JSON.stringify({ version: '4.2.2', features: { cloudAuth: true, remoteClient: true, remoteControl: true } }), '4.2.2')).not.toThrow();
  expect(() => checkAppImageCli(JSON.stringify({ version: '4.2.2', features: { remoteControl: false } }), '4.2.2')).toThrow('headless');
});
