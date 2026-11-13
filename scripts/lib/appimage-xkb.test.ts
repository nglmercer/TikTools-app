import { chmod, copyFile, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { expect, test } from 'bun:test';
import { verifyAppImageWebKit, verifyAppImageXkb } from './verify-installers.ts';

const runtimes = ['libxkbcommon-x11.so.0', 'libxkbcommon.so.0', 'libxcb-xkb.so.1'];

test.skipIf(process.platform !== 'linux' || !Bun.which('cc') || !Bun.which('readelf'))('AppImage rejects missing dlopen XKB libraries, host symlinks and missing RPATH', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-appimage-xkb-'));
  const libDir = join(root, 'usr/lib');
  const compile = (path: string, rpath: boolean) => {
    const result = Bun.spawnSync(['cc', '-shared', '-nostdlib', '-x', 'c', '/dev/null', '-o', path,
      ...(rpath ? ['-Wl,-rpath,$ORIGIN'] : [])], { stdout: 'pipe', stderr: 'pipe' });
    if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  };
  try {
    await mkdir(libDir, { recursive: true });
    const fixture = join(root, 'runtime.so');
    compile(fixture, true);
    for (const name of runtimes) await copyFile(fixture, join(libDir, name));
    await expect(verifyAppImageXkb(root)).resolves.toBeUndefined();
    for (const name of runtimes) {
      const path = join(libDir, name);
      await rm(path);
      await expect(verifyAppImageXkb(root)).rejects.toThrow(`missing bundled XKB runtime ${name}`);
      await symlink(fixture, path);
      await expect(verifyAppImageXkb(root)).rejects.toThrow(`missing bundled XKB runtime ${name}`);
      await rm(path);
      await writeFile(path, 'not an ELF library');
      await expect(verifyAppImageXkb(root)).rejects.toThrow(`missing bundled XKB runtime ${name}`);
      compile(path, false);
      await expect(verifyAppImageXkb(root)).rejects.toThrow(`${name} has no sibling-library RPATH`);
      await copyFile(fixture, path);
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});

// Opt in inside a disposable Ubuntu container with cc, pkg-config and
// libxkbcommon-x11-dev installed. The helpers/codecs are tiny ELF fixtures;
// the keyboard libraries and dependency deployment are real. No app build.
const linuxdeploy = process.env.TIKTOOLS_XKB_TEST_LINUXDEPLOY;
test.skipIf(process.platform !== 'linux' || !linuxdeploy)('linuxdeploy plugin bundles the real dlopen XKB seed and its xcb-xkb dependency', async () => {
  const root = await mkdtemp(join(tmpdir(), 'tiktools-xkb-plugin-'));
  // The production plugin deliberately requires WebKit helpers under /usr.
  const hostLib = await mkdtemp('/usr/lib/tiktools-xkb-fixture-');
  const appDir = join(root, 'AppDir');
  const helperDir = join(hostLib, 'webkit2gtk-4.1');
  const tools = join(root, 'tools');
  const mockBin = join(root, 'bin');
  const run = (args: string[], env = process.env) => {
    const result = Bun.spawnSync(args, { env, stdout: 'pipe', stderr: 'pipe', timeout: 30_000 });
    if (result.exitCode !== 0) throw new Error(result.stdout.toString() + result.stderr.toString());
  };
  try {
    for (const dir of [join(appDir, 'usr/lib'), join(helperDir, 'injected-bundle'), tools, mockBin]) {
      await mkdir(dir, { recursive: true });
    }
    const source = join(root, 'webkit.c');
    await writeFile(source, `const char *helper_path = ${JSON.stringify(helperDir)};\n`);
    run(['cc', '-shared', '-nostdlib', source, '-o', join(appDir, 'usr/lib/libwebkit2gtk-4.1.so.0')]);
    for (const helper of ['WebKitNetworkProcess', 'WebKitWebProcess', 'injected-bundle/libwebkit2gtkinjectedbundle.so']) {
      await copyFile('/bin/true', join(helperDir, helper));
    }
    const plugin = join(tools, 'linuxdeploy-plugin-tiktoolswebkit.sh');
    await copyFile(resolve(import.meta.dir, '../appimage/linuxdeploy-plugin-tiktoolswebkit.sh'), plugin);
    await copyFile(resolve(import.meta.dir, '../appimage/gstreamer-plugins.txt'), join(tools, 'gstreamer-plugins.txt'));
    await symlink(resolve(linuxdeploy!), join(tools, 'linuxdeploy-x86_64.AppImage'));
    const plugins = (await readFile(join(tools, 'gstreamer-plugins.txt'), 'utf8')).trim().split(/\s+/);
    for (const name of plugins) await copyFile('/bin/true', join(hostLib, `libgst${name}.so`));
    await copyFile('/bin/true', join(hostLib, 'gst-plugin-scanner'));
    await writeFile(join(mockBin, 'pkg-config'), `#!/bin/sh
case "$*" in
  '--variable=libdir webkit2gtk-4.1'|'--variable=pluginsdir gstreamer-1.0'|'--variable=pluginscannerdir gstreamer-1.0')
    printf '%s\\n' '${hostLib}' ;;
  *) exec /usr/bin/pkg-config "$@" ;;
esac
`);
    await chmod(join(mockBin, 'pkg-config'), 0o755);
    const env = { ...process.env, PATH: `${mockBin}:${process.env.PATH}`, APPIMAGE_EXTRACT_AND_RUN: '1', NO_CLEANUP: '1', NO_STRIP: '1' };
    run(['bash', plugin, '--appdir', appDir], env);
    await verifyAppImageWebKit(appDir);
    await expect(verifyAppImageXkb(appDir)).resolves.toBeUndefined();
    // Removing the transitive dependency must fail even on this build host,
    // where apt installed a perfectly usable system copy.
    await rm(join(appDir, 'usr/lib/libxcb-xkb.so.1'));
    await expect(verifyAppImageXkb(appDir)).rejects.toThrow('missing bundled XKB runtime libxcb-xkb.so.1');
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(hostLib, { recursive: true, force: true });
  }
}, 45_000);
