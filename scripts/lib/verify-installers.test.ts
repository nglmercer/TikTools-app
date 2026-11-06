import { copyFile, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { describe, expect, test } from 'bun:test';
import { verifyAppImageWebKit, verifyDebDependencies, verifyLinuxDesktop } from './verify-installers.ts';

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
    } finally { await rm(root, { recursive: true, force: true }); }
  });
});
