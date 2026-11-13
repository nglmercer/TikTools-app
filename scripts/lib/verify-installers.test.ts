import { chmod, copyFile, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { describe, expect, test } from 'bun:test';
import { verifyAppImageWebKit, verifyDebDependencies, verifyInstaller, verifyLinuxDesktop } from './verify-installers.ts';

test.skipIf(process.platform !== 'linux')('DEB verifies the current bundle without bundled plugins and rejects changed web assets', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-deb-bundle-'));
  const staging = join(root, 'staging');
  const data = join(root, 'data');
  const resources = join(data, 'usr/lib/tiktools-desktop');
  const control = join(root, 'control');
  const run = (...cmd: string[]) => {
    const result = Bun.spawnSync(cmd, { cwd: root, stdout: 'pipe', stderr: 'pipe' });
    if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  };
  try {
    for (const directory of [join(staging, 'web'), join(resources, 'web'), control,
      join(data, 'usr/bin'), join(data, 'usr/share/applications'), join(data, 'usr/share/icons/hicolor/256x256/apps')]) {
      await mkdir(directory, { recursive: true });
    }
    for (const file of ['web/index.html', 'LICENSE', 'README.md']) {
      await writeFile(join(staging, file), file);
      await copyFile(join(staging, file), join(resources, file));
    }
    const binary = join(staging, 'tiktools-desktop');
    await writeFile(binary, '#!/bin/sh\nprintf "4.2.0\\n"\n');
    await chmod(binary, 0o755);
    await copyFile(binary, join(data, 'usr/bin/tiktools-desktop'));
    await writeFile(join(data, 'usr/share/applications/tiktools-desktop.desktop'), '[Desktop Entry]\nType=Application\nExec=tiktools-desktop\nIcon=tiktools-desktop\n');
    await copyFile(resolve(import.meta.dir, '../../assets/tiktools-icon.png'), join(data, 'usr/share/icons/hicolor/256x256/apps/tiktools-desktop.png'));
    await writeFile(join(control, 'control'), 'Depends: libgtk-3-0, libwebkit2gtk-4.1-0\n');
    await writeFile(join(root, 'debian-binary'), '2.0\n');
    const pack = () => {
      run('tar', '-czf', 'control.tar.gz', '-C', control, 'control');
      run('tar', '-czf', 'data.tar.gz', '-C', data, 'usr');
      run('ar', 'rc', 'fixture.deb', 'debian-binary', 'control.tar.gz', 'data.tar.gz');
    };
    pack();
    await expect(verifyInstaller(staging, join(root, 'fixture.deb'), 'deb', '4.2.0')).resolves.toBeUndefined();
    await writeFile(join(resources, 'web/index.html'), 'changed');
    pack();
    await expect(verifyInstaller(staging, join(root, 'fixture.deb'), 'deb', '4.2.0')).rejects.toThrow('installer content mismatch: web/index.html');
  } finally { await rm(root, { recursive: true, force: true }); }
});

describe('Linux installer metadata', () => {
  test('requires GTK and WebKitGTK Debian dependencies', () => {
    expect(() => verifyDebDependencies('libgtk-3-0 | libgtk-3-0t64, libwebkit2gtk-4.1-0 | libwebkit2gtk-4.1-0t64')).not.toThrow();
    expect(() => verifyDebDependencies('libgtk-3-0, libc6')).toThrow();
    expect(() => verifyDebDependencies('not-libgtk-3-0, libwebkit2gtk-4.1-0')).toThrow();
  });
  test('requires a matching desktop entry and application icon', async () => {
    const root = await mkdtemp(join(tmpdir(), 'tiktools-linux-package-'));
    try {
      const applications = join(root, 'usr/share/applications');
      const icons = join(root, 'usr/share/icons/hicolor/256x256/apps');
      await mkdir(applications, { recursive: true });
      await mkdir(icons, { recursive: true });
      const desktop = join(applications, 'tiktools-desktop.desktop');
      await writeFile(desktop, '[Desktop Entry]\nType=Application\nExec=tiktools-desktop\nIcon=tiktools-desktop\n');
      await expect(verifyLinuxDesktop(root)).rejects.toThrow();
      await copyFile(resolve(import.meta.dir, '../../assets/tiktools-icon.png'), join(icons, 'tiktools-desktop.png'));
      await expect(verifyLinuxDesktop(root)).resolves.toBeUndefined();
      await writeFile(desktop, '[Desktop Entry]\nType=Application\nExec=other\nIcon=tiktools-desktop\n');
      await expect(verifyLinuxDesktop(root)).rejects.toThrow();
    } finally { await rm(root, { recursive: true, force: true }); }
  });
  test('requires AppDir-relative WebKit helpers in the AppImage', async () => {
    const root = await mkdtemp(join(tmpdir(), 'tiktools-appimage-webkit-'));
    const elf = Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0]);
    const library = join(root, 'usr/lib/libwebkit2gtk-4.1.so.0');
    const helpers = join(root, 'usr/lib/x86_64-linux-gnu/webkit2gtk-4.1');
    try {
      await mkdir(join(helpers, 'injected-bundle'), { recursive: true });
      await writeFile(library, Buffer.concat([elf, Buffer.from('/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1\0')]));
      await expect(verifyAppImageWebKit(root)).rejects.toThrow('host WebKit helpers');
      await writeFile(library, Buffer.concat([elf, Buffer.from('././/lib/x86_64-linux-gnu/webkit2gtk-4.1\0')]));
      await expect(verifyAppImageWebKit(root)).rejects.toThrow('missing WebKit helper');
      for (const helper of ['WebKitNetworkProcess', 'WebKitWebProcess', 'injected-bundle/libwebkit2gtkinjectedbundle.so']) {
        await writeFile(join(helpers, helper), elf);
      }
      await expect(verifyAppImageWebKit(root)).resolves.toBeUndefined();
      await writeFile(join(root, 'usr/lib/libwayland-client.so.0'), '');
      await expect(verifyAppImageWebKit(root)).rejects.toThrow('must not bundle libwayland-client.so.0');
    } finally { await rm(root, { recursive: true, force: true }); }
  });
});
