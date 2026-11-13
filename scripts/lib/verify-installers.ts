import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, mkdtemp, open, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';

async function run(args: string[], cwd: string, timeoutMs = 600_000): Promise<string> {
  const child = Bun.spawn(args, { cwd, stdout: 'pipe', stderr: 'pipe' });
  const timer = setTimeout(() => child.kill(), timeoutMs);
  const [output, errors, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]).finally(() => clearTimeout(timer));
  if (code !== 0) throw new Error(`${basename(args[0] as string)} failed: ${errors.slice(-2000)}`);
  return output;
}

async function files(root: string, relative = ''): Promise<string[]> {
  const entries = await readdir(join(root, relative), { withFileTypes: true });
  const results: string[] = [];
  for (const entry of entries) {
    const path = join(relative, entry.name);
    if (entry.isDirectory()) results.push(...await files(root, path));
    else if (entry.isFile()) results.push(path);
    else throw new Error(`unsupported staged file type: ${path}`);
  }
  return results;
}

async function digest(path: string): Promise<string> {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk as Buffer);
  return hash.digest('hex');
}

async function isElf(path: string): Promise<boolean> {
  const file = await open(path, 'r');
  try {
    const header = Buffer.alloc(4);
    const { bytesRead } = await file.read(header, 0, 4, 0);
    return bytesRead === 4 && header.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]));
  } finally { await file.close(); }
}

async function matchesPatchedElf(source: string, packaged: string): Promise<boolean> {
  if (!await isElf(source) || !await isElf(packaged)) return false;
  const [before, after] = await Promise.all([run(['readelf', '-n', source], tmpdir()), run(['readelf', '-n', packaged], tmpdir())]);
  const buildId = (text: string) => text.match(/Build ID:\s*([0-9a-f]+)/i)?.[1]?.toLowerCase();
  const id = buildId(before);
  return !!id && id === buildId(after);
}

async function checkContents(staging: string, resources: string, binary: string, version: string, appimage: boolean): Promise<void> {
  for (const relative of ['web']) {
    const entries = await files(join(staging, relative));
    if (entries.length === 0) throw new Error(`staged ${relative} is empty`);
    for (const entry of entries) {
      const source = join(staging, relative, entry);
      const packaged = join(resources, relative, entry);
      if ((await lstat(packaged)).isSymbolicLink() ||
        (await digest(source) !== await digest(packaged) && !(appimage && await matchesPatchedElf(source, packaged)))) {
        throw new Error(`installer content mismatch: ${relative}/${entry}`);
      }
    }
  }
  for (const file of ['LICENSE', 'README.md']) {
    if (await digest(join(staging, file)) !== await digest(join(resources, file))) throw new Error(`installer content mismatch: ${file}`);
  }
  const stagedBinary = join(staging, basename(binary));
  if (await digest(stagedBinary) !== await digest(binary) && !(appimage && await matchesPatchedElf(stagedBinary, binary))) throw new Error('installer binary differs from staged binary');
  if ((await run([binary, '--version'], resources, 15_000)).trim() !== version) throw new Error('installer binary version does not match the release');
}

/** Validate Linux desktop integration from the extracted package tree. */
export async function verifyLinuxDesktop(root: string): Promise<void> {
  const desktop = await readFile(join(root, 'usr/share/applications/tiktools-desktop.desktop'), 'utf8');
  if (!/^\[Desktop Entry\]\s*$/m.test(desktop) || !/^Type=Application\s*$/m.test(desktop) ||
      !/^Exec=tiktools-desktop(?:\s.*)?$/m.test(desktop) || !/^Icon=tiktools-desktop\s*$/m.test(desktop)) {
    throw new Error('invalid Linux desktop entry');
  }
  const iconRoot = join(root, 'usr/share/icons/hicolor');
  const expected = await digest(resolve(import.meta.dir, '../../assets/tiktools-icon.png'));
  let found = false;
  for await (const relative of new Bun.Glob('*/apps/tiktools-desktop.png').scan({ cwd: iconRoot, onlyFiles: true })) {
    const icon = join(iconRoot, relative);
    if ((await lstat(icon)).isFile() && await digest(icon) === expected) found = true;
  }
  if (!found) throw new Error('Linux package is missing the application icon');
}

/**
 * Ensure the AppImage's bundled WebKitGTK spawns its bundled helpers. The
 * library hardcodes the build host's helper directory, which does not exist on
 * other distributions (e.g. Arch keeps it in /usr/lib/webkit2gtk-4.1), so the
 * packaging plugin rewrites "/usr" to the same-length "././" that AppRun resolves from
 * "$APPDIR/usr". A smoke launch on the build host cannot catch this.
 */
export async function verifyAppImageWebKit(appDir: string): Promise<void> {
  const libDir = join(appDir, 'usr', 'lib');
  const libraries: string[] = [];
  for (const name of await readdir(libDir)) {
    if (name.startsWith('libwebkit2gtk-4.1.so') && (await lstat(join(libDir, name))).isFile()) libraries.push(name);
  }
  if (libraries.length === 0) throw new Error('AppImage is missing the bundled libwebkit2gtk-4.1');
  for (const name of libraries) {
    const text = (await readFile(join(libDir, name))).toString('latin1');
    const absolute = text.match(/\/usr\/lib(?:\/[\w.-]+)?\/webkit2gtk-4\.1(?=[/\0])/);
    if (absolute) throw new Error(`AppImage ${name} still references host WebKit helpers at ${absolute[0]}`);
    const relative = text.match(/\.\/\.\/\/(lib(?:\/[\w.-]+)?\/webkit2gtk-4\.1)(?=[/\0])/)?.[1];
    if (!relative) throw new Error(`AppImage ${name} has no AppDir-relative WebKit helper path`);
    for (const helper of ['WebKitNetworkProcess', 'WebKitWebProcess', 'injected-bundle/libwebkit2gtkinjectedbundle.so']) {
      const path = join(appDir, 'usr', relative, helper);
      if (!await lstat(path).then((stat) => stat.isFile(), () => false) || !await isElf(path)) {
        throw new Error(`AppImage is missing WebKit helper usr/${relative}/${helper}`);
      }
    }
  }
  const requiredPlugins = (await readFile(resolve(import.meta.dir, '../appimage/gstreamer-plugins.txt'), 'utf8')).trim().split(/\s+/);
  for (const plugin of requiredPlugins) {
    const path = join(libDir, 'gstreamer-1.0', `libgst${plugin}.so`);
    if (!await isElf(path).catch(() => false)) throw new Error(`AppImage is missing GStreamer plugin ${plugin}`);
  }
  if (!await isElf(join(appDir, 'usr/libexec/gstreamer-1.0/gst-plugin-scanner')).catch(() => false)) {
    throw new Error('AppImage is missing GStreamer plugin scanner');
  }
  // The host's Wayland client stack must win over any bundled copy: a pinned
  // build-host version shadows the host libraries and makes Mesa's EGL fail
  // inside WebKitWebProcess on rolling distributions (EGL_BAD_PARAMETER).
  for (const name of await readdir(libDir)) {
    if (/^libwayland-(?:client|cursor|egl|server)\.so/.test(name)) {
      throw new Error(`AppImage must not bundle ${name}; the host Wayland client stack is required`);
    }
  }
}

/** dlopen-only X11 keyboard libraries must resolve from the AppImage itself. */
export async function verifyAppImageXkb(appDir: string): Promise<void> {
  const libDir = join(appDir, 'usr', 'lib');
  for (const name of ['libxkbcommon-x11.so.0', 'libxkbcommon.so.0', 'libxcb-xkb.so.1']) {
    const path = join(libDir, name);
    // Require packaged files: a symlink to a host library would hide this gap.
    if (!await lstat(path).then((stat) => stat.isFile(), () => false) || !await isElf(path)) {
      throw new Error(`AppImage is missing bundled XKB runtime ${name}`);
    }
    const dynamic = await run(['readelf', '-d', path], appDir, 15_000);
    const paths = [...dynamic.matchAll(/\((?:RPATH|RUNPATH)\)[^\n]*\[([^\]]*)\]/g)]
      .flatMap((match) => match[1]!.split(':'));
    if (!paths.some((entry) => entry === '$ORIGIN' || entry === '$ORIGIN/.')) {
      throw new Error(`AppImage XKB runtime ${name} has no sibling-library RPATH`);
    }
  }
}

/** Check Debian-family runtime names without assuming other distributions use them. */
export function verifyDebDependencies(dependencies: string): void {
  for (const name of ['libgtk-3-0', 'libwebkit2gtk-4.1-0']) {
    const escaped = name.replaceAll('.', '\\.');
    if (!new RegExp(`(?:^|[ ,|])${escaped}(?:t64)?(?:[ ,|(]|$)`).test(dependencies)) {
      throw new Error(`DEB package is missing ${name} dependency`);
    }
  }
}

/** Inspect extracted installer contents against the already verified staging tree. */
export async function verifyInstaller(staging: string, installer: string, format: 'deb' | 'appimage' | 'nsis', version: string): Promise<void> {
  staging = resolve(staging);
  installer = resolve(installer);
  const temp = await mkdtemp(join(tmpdir(), 'tiktools-installer-check-'));
  try {
    if (format === 'deb') {
      await run(['ar', 'x', installer], temp);
      const control = await run(['tar', '-xOzf', 'control.tar.gz', 'control'], temp);
      const depends = control.match(/^Depends:\s*(.*)$/m)?.[1];
      if (!depends) throw new Error('DEB package is missing dependency metadata');
      verifyDebDependencies(depends);
      await run(['tar', '-xzf', 'data.tar.gz', '--no-same-owner', '-C', temp], temp);
      await verifyLinuxDesktop(temp);
      await checkContents(staging, join(temp, 'usr', 'lib', 'tiktools-desktop'), join(temp, 'usr', 'bin', 'tiktools-desktop'), version, false);
    } else if (format === 'appimage') {
      await run([installer, '--appimage-extract'], temp);
      await verifyLinuxDesktop(join(temp, 'squashfs-root'));
      await verifyAppImageWebKit(join(temp, 'squashfs-root'));
      await verifyAppImageXkb(join(temp, 'squashfs-root'));
      const root = join(temp, 'squashfs-root', 'usr');
      // linuxdeploy patches the ELF RPATH, so compare its reported version.
      await checkContents(staging, join(root, 'lib', 'tiktools-desktop'), join(root, 'bin', 'tiktools-desktop'), version, true);
    } else {
      const sevenZip = Bun.which('7z') ?? join(process.env.ProgramFiles || 'C:\\Program Files', '7-Zip', '7z.exe');
      await run([sevenZip, 'x', '-y', `-o${temp}`, installer], temp);
      const matches = (await files(temp)).filter((name) => basename(name).toLowerCase() === 'tiktools-desktop.exe');
      if (matches.length !== 1) throw new Error('NSIS archive must contain exactly one desktop executable');
      const binary = join(temp, matches[0] as string);
      await checkContents(staging, join(binary, '..'), binary, version, false);
    }
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}
