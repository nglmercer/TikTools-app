import { chmod, cp, mkdir, readFile, rm, stat } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { napiTargetFromRustTarget, resolveRustTarget } from './lib/plugin-targets.ts';

const repositoryRoot = resolve(import.meta.dir, '..');
const cargoWrapper = join(repositoryRoot, 'scripts', 'cargo-with-linker.mjs');
const supportedPlatforms = {
  'windows-x86_64': {
    archiveExtension: '.zip',
    binaryName: 'tiktools-desktop.exe',
    rustTarget: 'x86_64-pc-windows-msvc',
  },
  'linux-x86_64': {
    archiveExtension: '.tar.gz',
    binaryName: 'tiktools-desktop',
    rustTarget: 'x86_64-unknown-linux-gnu',
  },
  'macos-arm64': {
    archiveExtension: '.tar.gz',
    binaryName: 'tiktools-desktop',
    rustTarget: 'aarch64-apple-darwin',
  },
  'macos-x86_64': {
    archiveExtension: '.tar.gz',
    binaryName: 'tiktools-desktop',
    rustTarget: 'x86_64-apple-darwin',
  },
} as const;

/**
 * Official built-in plugins bundled with every release.
 *
 * `runtime: 'napi-vm'` entries ship a TypeScript guest plus the release
 * target's native bindings, fetched from each library's pinned provider
 * (`nativeLibs` + `native-libs.lock.json` in the example directory) and
 * verified by the shared stage-native tool. Content pins live in the
 * lockfile; this script only names the expected staged layout for the
 * archive verification below.
 */
type BundledPlugin =
  | { id: string; example: string; entry: string; runtime: 'process' }
  | {
      id: string;
      example: string;
      entry: string;
      runtime: 'napi-vm';
      native: { package: string; binary: string };
    };
const BUNDLED_PLUGINS: readonly BundledPlugin[] = [
  {
    id: 'hotkeys',
    example: 'hotkey-napi-plugin',
    entry: 'dist/index.js',
    runtime: 'napi-vm',
    native: { package: 'rdev-node', binary: 'node-rdev' },
  },
];

type ReleasePlatform = keyof typeof supportedPlatforms;

function fail(message: string): never {
  throw new Error(`Release packaging failed: ${message}`);
}

function requiredEnvironment(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) fail(`${name} is required`);
  return value;
}

function run(command: string, args: string[]): string {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd: repositoryRoot,
    stdout: 'pipe',
    stderr: 'inherit',
  });
  const output = result.stdout ? new TextDecoder().decode(result.stdout) : '';
  if (!result.success) fail(`${command} ${args.join(' ')} exited with code ${result.exitCode}`);
  return output;
}

async function isFile(path: string): Promise<boolean> {
  const info = await stat(path).catch(() => null);
  return info?.isFile() ?? false;
}

function isSupportedPlatform(value: string): value is ReleasePlatform {
  return Object.hasOwn(supportedPlatforms, value);
}

/** Plugin-relative files each bundled entry must contribute to the archive. */
function bundledExpectedFiles(bundled: BundledPlugin, releasePlatform: ReleasePlatform): string[] {
  if (bundled.runtime === 'napi-vm') {
    const triple = napiTargetFromRustTarget(supportedPlatforms[releasePlatform].rustTarget);
    const binding = `${bundled.native.binary}.${triple}.node`;
    return [
      'plugin.json',
      bundled.entry,
      `node_modules/${bundled.native.package}/package.json`,
      `node_modules/${bundled.native.package}/index.js`,
      `node_modules/${bundled.native.package}/index.d.ts`,
      `node_modules/${bundled.native.package}/${binding}`,
    ];
  }
  return ['plugin.json', `${bundled.entry}${resolveRustTarget(platform.rustTarget).executableExtension}`];
}

/**
 * Stage a napi-vm plugin: compile the TypeScript guest, then fetch every
 * declared native library for the release target from its pinned provider
 * through the shared stage-native tool, so release layout matches the
 * dev/packaged flows byte for byte.
 */
async function stageNapiVmPlugin(
  bundled: Extract<BundledPlugin, { runtime: 'napi-vm' }>,
  exampleDirectory: string,
  pluginDirectory: string,
  releasePlatform: ReleasePlatform,
): Promise<void> {
  const tsc = join(repositoryRoot, 'node_modules', 'typescript', 'bin', 'tsc');
  if (!(await isFile(tsc))) {
    fail(`TypeScript compiler is missing at ${tsc}; run bun ci first`);
  }
  run('bun', [tsc, '-p', exampleDirectory]);
  const builtEntry = join(exampleDirectory, bundled.entry);
  if (!(await isFile(builtEntry))) {
    fail(`tsc built ${bundled.id}, but its entry was not found at ${builtEntry}`);
  }
  const manifestPath = join(exampleDirectory, 'plugin.json');
  const lockfilePath = join(exampleDirectory, 'native-libs.lock.json');
  if (!(await isFile(lockfilePath))) {
    fail(`bundled plugin ${bundled.id} declares nativeLibs but has no committed ${lockfilePath}`);
  }
  await mkdir(pluginDirectory, { recursive: true });
  run('node', [
    cargoWrapper,
    'run',
    '--release',
    '--locked',
    '-p',
    'tiktools-plugin-sdk',
    '--features',
    'providers',
    '--bin',
    'tiktools-plugin-stage-native',
    '--',
    '--provider',
    'all',
    '--manifest',
    manifestPath,
    '--plugin-dir',
    pluginDirectory,
    '--lockfile',
    lockfilePath,
    '--target',
    napiTargetFromRustTarget(supportedPlatforms[releasePlatform].rustTarget),
    '--overwrite',
  ]);
  await cp(join(exampleDirectory, 'plugin.json'), join(pluginDirectory, 'plugin.json'));
  await mkdir(join(pluginDirectory, 'dist'), { recursive: true });
  await cp(builtEntry, join(pluginDirectory, bundled.entry));
}

const tag = requiredEnvironment('RELEASE_TAG');
if (!/^v[^/]+$/.test(tag)) fail(`RELEASE_TAG must be a Git tag such as v0.1.0, got ${tag}`);
const version = tag.slice(1);
const platformValue = requiredEnvironment('RELEASE_PLATFORM');
if (!isSupportedPlatform(platformValue)) {
  fail(`RELEASE_PLATFORM must be one of ${Object.keys(supportedPlatforms).join(', ')}`);
}
const platform = supportedPlatforms[platformValue];

const webRoot = resolve(repositoryRoot, 'dist', 'web');
const webIndex = join(webRoot, 'index.html');
if (!(await isFile(webIndex))) {
  fail(`frontend output is missing ${webIndex}; run bun run build:web first`);
}

const binaryPath = resolve(
  repositoryRoot,
  process.env.RELEASE_BINARY?.trim() || join('target', 'release', platform.binaryName),
);
if (!(await isFile(binaryPath))) {
  fail(`compiled desktop binary is missing ${binaryPath}`);
}

const releaseDirectory = resolve(repositoryRoot, 'release');
const stagingDirectory = join(releaseDirectory, 'staging');
const bundleName = 'TikTools';
const bundleDirectory = join(stagingDirectory, bundleName);
const archiveName = `TikTools-${version}-${platformValue}${platform.archiveExtension}`;
const archivePath = join(releaseDirectory, archiveName);

await rm(bundleDirectory, { recursive: true, force: true });
await rm(archivePath, { force: true });
await mkdir(bundleDirectory, { recursive: true });

await cp(binaryPath, join(bundleDirectory, platform.binaryName));
await mkdir(join(bundleDirectory, 'plugins'), { recursive: true });
// Global Hotkeys is an official feature: build each bundled plugin for the
// release target and stage manifest + executable. A missing built-in
// plugin fails packaging loudly instead of shipping a hotkey-less app.
const pluginTarget = resolveRustTarget(platform.rustTarget);
for (const bundled of BUNDLED_PLUGINS) {
  const exampleDirectory = join(repositoryRoot, 'examples', bundled.example);
  const manifestPath = join(exampleDirectory, 'plugin.json');
  if (!(await isFile(manifestPath))) {
    fail(`bundled plugin manifest is missing ${manifestPath}`);
  }
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as { id?: unknown };
  if (manifest.id !== bundled.id) {
    fail(`${manifestPath} declares id ${String(manifest.id)}, expected ${bundled.id}`);
  }
  if (bundled.runtime === 'napi-vm') {
    await stageNapiVmPlugin(
      bundled,
      exampleDirectory,
      join(bundleDirectory, 'plugins', bundled.id),
      platformValue,
    );
    continue;
  }
  run('node', [
    cargoWrapper,
    'build',
    '--release',
    '--locked',
    '--target',
    pluginTarget.rustTarget,
    '--manifest-path',
    join(exampleDirectory, 'Cargo.toml'),
  ]);
  const builtEntryName = `${bundled.entry}${pluginTarget.executableExtension}`;
  const builtEntryPath = join(
    exampleDirectory,
    'target',
    pluginTarget.rustTarget,
    'release',
    builtEntryName,
  );
  if (!(await isFile(builtEntryPath))) {
    fail(`cargo built ${bundled.id}, but its entry was not found at ${builtEntryPath}`);
  }
  const pluginDirectory = join(bundleDirectory, 'plugins', bundled.id);
  await mkdir(pluginDirectory, { recursive: true });
  await cp(manifestPath, join(pluginDirectory, 'plugin.json'));
  const stagedEntry = join(pluginDirectory, builtEntryName);
  await cp(builtEntryPath, stagedEntry);
  if (pluginTarget.executableExtension === '') {
    await chmod(stagedEntry, 0o755);
  }
}
await cp(webRoot, join(bundleDirectory, 'web'), { recursive: true });
for (const file of ['LICENSE', 'README.md']) {
  const source = resolve(repositoryRoot, file);
  if (!(await isFile(source))) fail(`release documentation is missing ${source}`);
  await cp(source, join(bundleDirectory, file));
}

if (platformValue === 'windows-x86_64') {
  run('tar', ['-a', '-c', '-f', archivePath, '-C', stagingDirectory, bundleName]);
} else {
  run('tar', ['-czf', archivePath, '-C', stagingDirectory, bundleName]);
}

const archiveListing = platformValue === 'windows-x86_64'
  ? run('tar', ['-tf', archivePath])
  : run('tar', ['-tzf', archivePath]);
const listingEntries = archiveListing.split(/\r?\n/).map((entry) => entry.replaceAll('\\', '/'));
const expectedEntries = [
  `${bundleName}/${platform.binaryName}`,
  `${bundleName}/web/index.html`,
  `${bundleName}/LICENSE`,
  `${bundleName}/README.md`,
  ...BUNDLED_PLUGINS.flatMap((bundled) =>
    bundledExpectedFiles(bundled, platformValue).map(
      (relative) => `${bundleName}/plugins/${bundled.id}/${relative}`,
    ),
  ),
];
for (const expectedEntry of expectedEntries) {
  if (!listingEntries.some((entry) => entry === expectedEntry)) {
    fail(`archive ${archivePath} does not contain ${expectedEntry}`);
  }
}

// Verify the archive itself, not only the pre-archive staging directory. This
// catches layout regressions caused by the platform tar/ZIP implementation.
const extractedDirectory = join(stagingDirectory, 'verify-extracted');
await rm(extractedDirectory, { recursive: true, force: true });
await mkdir(extractedDirectory, { recursive: true });
run('tar', platformValue === 'windows-x86_64'
  ? ['-xf', archivePath, '-C', extractedDirectory]
  : ['-xzf', archivePath, '-C', extractedDirectory]);
const extractedBundle = join(extractedDirectory, bundleName);
for (const relative of [
  platform.binaryName,
  'web/index.html',
  'LICENSE',
  'README.md',
  ...BUNDLED_PLUGINS.flatMap((bundled) =>
    bundledExpectedFiles(bundled, platformValue).map(
      (relative) => `plugins/${bundled.id}/${relative}`,
    ),
  ),
]) {
  if (!(await isFile(join(extractedBundle, relative)))) {
    fail(`extracted archive is missing ${bundleName}/${relative}`);
  }
}

console.log(`Created ${basename(archivePath)}`);
console.log(`Package root: ${bundleDirectory}`);
console.log(`Archive: ${archivePath}`);
