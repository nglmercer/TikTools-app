/** Package the already verified portable staging tree without rebuilding it. */
import { constants } from 'node:fs';
import { chmod, copyFile, lstat, mkdir, mkdtemp, readdir, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { normalizeVersion } from './bump-version.ts';
import { verifyInstaller } from './lib/verify-installers.ts';

const root = resolve(import.meta.dir, '..');
const tag = process.env.RELEASE_TAG;
const platform = process.env.RELEASE_PLATFORM;
if (!tag?.startsWith('v')) throw new Error('RELEASE_TAG must be a v-prefixed SemVer tag');
const version = normalizeVersion(tag);
if (platform !== 'linux-x86_64' && platform !== 'windows-x86_64') throw new Error('unsupported RELEASE_PLATFORM');
if ((platform === 'linux-x86_64' && (process.platform !== 'linux' || process.arch !== 'x64')) ||
    (platform === 'windows-x86_64' && (process.platform !== 'win32' || process.arch !== 'x64'))) {
  throw new Error('installer packaging must run on the matching OS and architecture');
}

async function run(args: string[], env: Record<string, string | undefined> = process.env): Promise<void> {
  const child = Bun.spawn(args, { cwd: root, env, stdin: 'inherit', stdout: 'inherit', stderr: 'inherit' });
  if (await child.exited !== 0) throw new Error(`command failed: ${args.join(' ')}`);
}

const releaseDir = join(root, 'release');
const staging = join(releaseDir, 'staging', 'TikTools');
const binary = join(staging, platform === 'windows-x86_64' ? 'tiktools-desktop.exe' : 'tiktools-desktop');
await run([process.execPath, 'scripts/check-version.ts', '--binary'], { ...process.env, RELEASE_BINARY: binary });
const packager = Bun.spawn(['cargo', 'packager', '--version'], { cwd: root, stdout: 'pipe', stderr: 'pipe' });
const [packagerVersion, packagerExit] = await Promise.all([new Response(packager.stdout).text(), packager.exited]);
if (packagerExit !== 0 || packagerVersion.trim() !== 'cargo-packager 0.11.8') {
  throw new Error('cargo-packager 0.11.8 is required');
}
let appImageTools: string | undefined;
if (platform === 'linux-x86_64') {
  // cargo-packager only downloads linuxdeploy plugins missing from its tool
  // cache (dirs::cache_dir()/.cargo-packager/AppImage), so always install the
  // reviewed in-repo plugin there instead of fetching it from the network.
  const cacheHome = process.env.XDG_CACHE_HOME?.startsWith('/') ? process.env.XDG_CACHE_HOME : join(homedir(), '.cache');
  const tools = join(cacheHome, '.cargo-packager', 'AppImage');
  const plugin = join(tools, 'linuxdeploy-plugin-tiktoolswebkit.sh');
  await mkdir(tools, { recursive: true });
  await rm(plugin, { force: true });
  await copyFile(join(root, 'scripts', 'appimage', 'linuxdeploy-plugin-tiktoolswebkit.sh'), plugin);
  await chmod(plugin, 0o755);
  await copyFile(join(root, 'scripts', 'appimage', 'gstreamer-plugins.txt'), join(tools, 'gstreamer-plugins.txt'));
  appImageTools = tools;
}
await mkdir(releaseDir, { recursive: true });
const output = await mkdtemp(join(releaseDir, '.packager-'));
try {
  const formats = platform === 'windows-x86_64' ? 'nsis' : 'deb,appimage';
  // linuxdeploy's bundled strip may not understand modern ELF RELR sections.
  const packagingEnv = { ...process.env, CARGO_PACKAGER_SIGN_PRIVATE_KEY: undefined, CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: undefined };
  // linuxdeploy only discovers plugins on PATH, not next to its own AppImage.
  await run(['cargo', 'packager', '--config', 'Packager.toml', '--formats', formats, '--out-dir', output],
    appImageTools ? { ...packagingEnv, NO_STRIP: '1', PATH: `${appImageTools}:${process.env.PATH ?? ''}` } : packagingEnv);
  const files = await readdir(output);
  const extensions = platform === 'windows-x86_64' ? ['.exe'] : ['.deb', '.AppImage'];
  const validated: Array<{ source: string; destination: string }> = [];
  for (const extension of extensions) {
    const matches = files.filter((file) => file.endsWith(extension));
    if (matches.length !== 1) throw new Error(`expected one ${extension} installer, found ${matches.length}`);
    const format = extension === '.exe' ? 'nsis' : extension === '.deb' ? 'deb' : 'appimage';
    await verifyInstaller(staging, join(output, matches[0] as string), format, version);
    const destination = join(releaseDir, `TikTools-${version}-${platform}-installer${extension}`);
    if (await lstat(destination).then(() => true, () => false)) throw new Error(`installer already exists: ${destination}`);
    validated.push({ source: join(output, matches[0] as string), destination });
  }
  for (const { source, destination } of validated) {
    await copyFile(source, destination, constants.COPYFILE_EXCL);
    console.log(`Created ${destination}`);
  }
} finally {
  await rm(output, { recursive: true, force: true });
}
