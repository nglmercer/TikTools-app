import { chmod, cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { GATEWAY_WIDGET_KINDS, ensureGatewayWidgetsStaged } from './lib/gateway-widgets.ts';
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
 * Mirrors the development staging set (`scripts/dev-plugins.ts`):
 * the release archive carries every plugin a fresh checkout
 * stages, so testers and release users see the same plugin
 * catalog as developers.
 *
 * - `declarative` examples ship the manifest alone: the host
 *   interprets every action, so no build runs.
 * - `process` examples compile their standalone Cargo manifest
 *   for the release target.
 * - `first-party` process packages compile their workspace
 *   backend crate and, when the manifest declares a `ui` entry,
 *   stage the built plugin UI beside it.
 * - `napi-vm` entries ship a TypeScript guest plus the release
 *   target's native bindings, fetched from each library's pinned
 *   provider (`nativeLibs` + `native-libs.lock.json` in the
 *   example directory) and verified by the shared stage-native
 *   tool. Content pins live in the lockfile; this script only
 *   names the expected staged layout for the archive verification
 *   below.
 */
type BundledPlugin =
  | { id: string; source: 'example'; example: string; runtime: 'declarative' }
  | { id: string; source: 'example'; example: string; runtime: 'process'; entry: string }
  | {
      id: string;
      source: 'example';
      example: string;
      runtime: 'napi-vm';
      entry: string;
      native: { package: string; binary: string };
    }
  | {
      id: string;
      source: 'first-party';
      package: string;
      crate: string;
      runtime: 'process';
      entry: string;
      uiEntry?: string;
    };
const BUNDLED_PLUGINS: readonly BundledPlugin[] = [
  {
    id: 'audio.play.process',
    source: 'example',
    example: 'audio-process-plugin',
    runtime: 'process',
    entry: 'tiktools-audio-process-plugin',
  },
  {
    id: 'hotkeys',
    source: 'example',
    example: 'hotkey-napi-plugin',
    runtime: 'napi-vm',
    entry: 'dist/index.js',
    native: { package: 'rdev-node', binary: 'node-rdev' },
  },
  {
    id: 'minecraft.server',
    source: 'example',
    example: 'minecraft-server',
    runtime: 'declarative',
  },
  {
    id: 'sonicboom.server',
    source: 'first-party',
    package: 'sonicboom',
    crate: 'sonicboom-backend',
    runtime: 'process',
    entry: 'backend/dist/sonicboom-backend',
    uiEntry: 'ui/dist/index.html',
  },
  {
    id: 'textintel',
    source: 'example',
    example: 'textintel-process-plugin',
    runtime: 'process',
    entry: 'tiktools-textintel',
  },
  {
    id: 'tiktools.event-gateway',
    source: 'example',
    example: 'event-gateway-process-plugin',
    runtime: 'process',
    entry: 'event-gateway',
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

async function isDirectory(path: string): Promise<boolean> {
  const info = await stat(path).catch(() => null);
  return info?.isDirectory() ?? false;
}

function isSupportedPlatform(value: string): value is ReleasePlatform {
  return Object.hasOwn(supportedPlatforms, value);
}

/** Plugin-relative files each bundled entry must contribute to the archive. */
function bundledExpectedFiles(bundled: BundledPlugin, releasePlatform: ReleasePlatform): string[] {
  const pluginTarget = resolveRustTarget(supportedPlatforms[releasePlatform].rustTarget);
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
  if (bundled.runtime === 'declarative') {
    return ['plugin.json'];
  }
  const entryName = `${basename(bundled.entry)}${pluginTarget.executableExtension}`;
  const entryDirectory = dirname(bundled.entry);
  const entryPath =
    entryDirectory === '.' ? entryName : `${entryDirectory}/${entryName}`;
  const expected = ['plugin.json', entryPath];
  if (bundled.source === 'first-party' && bundled.uiEntry) {
    expected.push(bundled.uiEntry);
  }
  if (bundled.id === 'tiktools.event-gateway') {
    // Widget bundles are mandatory gateway content: every staged
    // kind must be inside the archive, never a 404 at runtime.
    for (const kind of GATEWAY_WIDGET_KINDS) {
      expected.push(`dist/widgets/${kind}/index.html`);
    }
  }
  return expected;
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
    'tiktools-plugin-devkit',
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

/** Directories a staged plugin package copies verbatim when present. */
async function copyOptionalContent(sourceDirectory: string, destinationDirectory: string): Promise<void> {
  for (const directory of ['assets', 'dist', 'locales']) {
    const source = join(sourceDirectory, directory);
    if (await isDirectory(source)) {
      await cp(source, join(destinationDirectory, directory), { recursive: true });
    }
  }
}

/** Writes a staged manifest whose entry names the platform executable. */
async function writeStagedManifest(
  manifestPath: string,
  pluginDirectory: string,
  entryName: string,
): Promise<void> {
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as Record<string, unknown>;
  const staged = { ...manifest, entry: entryName };
  await writeFile(join(pluginDirectory, 'plugin.json'), `${JSON.stringify(staged, null, 2)}\n`, 'utf8');
}

/**
 * Stage a declarative plugin: the manifest alone ships, because the
 * host interprets every action. No build runs for any target.
 */
async function stageDeclarativePlugin(
  bundled: Extract<BundledPlugin, { runtime: 'declarative' }>,
  pluginDirectory: string,
): Promise<void> {
  const exampleDirectory = join(repositoryRoot, 'examples', bundled.example);
  const manifestPath = join(exampleDirectory, 'plugin.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  console.log(`Staging declarative plugin ${bundled.id}...`);
  await mkdir(pluginDirectory, { recursive: true });
  await writeFile(join(pluginDirectory, 'plugin.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  await copyOptionalContent(exampleDirectory, pluginDirectory);
}

/**
 * Stage a process plugin from `examples/`: compile its standalone
 * Cargo manifest for the release target, then stage the manifest
 * (entry rewritten to the platform executable name) and the binary.
 * The Event Gateway additionally builds and stages its mandatory
 * widget bundles before the example `dist/` is copied across.
 */
async function stageProcessExamplePlugin(
  bundled: Extract<BundledPlugin, { runtime: 'process' }>,
  pluginDirectory: string,
  pluginTarget: ReturnType<typeof resolveRustTarget>,
): Promise<void> {
  const exampleDirectory = join(repositoryRoot, 'examples', bundled.example);
  const manifestPath = join(exampleDirectory, 'plugin.json');
  if (bundled.id === 'tiktools.event-gateway') {
    // Widget bundles are mandatory gateway content: a fresh clone
    // must stage them with no manual sync step, and packaging
    // refuses to ship a gateway whose /widgets/* routes 404.
    await ensureGatewayWidgetsStaged(repositoryRoot);
  }
  console.log(`Building example plugin ${bundled.id} (${pluginTarget.rustTarget})...`);
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
  const entryName = `${bundled.entry}${pluginTarget.executableExtension}`;
  const builtEntryPath = join(
    exampleDirectory,
    'target',
    pluginTarget.rustTarget,
    'release',
    entryName,
  );
  if (!(await isFile(builtEntryPath))) {
    fail(`cargo built ${bundled.id}, but its entry was not found at ${builtEntryPath}`);
  }
  await mkdir(pluginDirectory, { recursive: true });
  await writeStagedManifest(manifestPath, pluginDirectory, entryName);
  await cp(builtEntryPath, join(pluginDirectory, entryName));
  if (pluginTarget.executableExtension === '') {
    await chmod(join(pluginDirectory, entryName), 0o755);
  }
  await copyOptionalContent(exampleDirectory, pluginDirectory);
}

/**
 * Stage a first-party plugin from `plugins/`: compile its workspace
 * backend crate for the release target, stage the built UI beside it
 * when the manifest declares one, and rewrite the manifest entry to
 * the platform executable name so the loader resolves the staged file.
 */
async function stageFirstPartyPlugin(
  bundled: Extract<BundledPlugin, { source: 'first-party' }>,
  pluginDirectory: string,
  pluginTarget: ReturnType<typeof resolveRustTarget>,
): Promise<void> {
  const packageDirectory = join(repositoryRoot, 'plugins', bundled.package);
  const manifestPath = join(packageDirectory, 'plugin.json');
  console.log(`Building first-party plugin ${bundled.id} (${pluginTarget.rustTarget})...`);
  run('node', [
    cargoWrapper,
    'build',
    '--release',
    '--locked',
    '--target',
    pluginTarget.rustTarget,
    '-p',
    bundled.crate,
  ]);
  // The staged entry keeps its manifest-relative path (for example
  // `backend/dist/sonicboom-backend`), matching the dev staging
  // layout byte for byte.
  const entryName = `${bundled.entry}${pluginTarget.executableExtension}`;
  const builtEntryPath = join(
    repositoryRoot,
    'target',
    pluginTarget.rustTarget,
    'release',
    `${bundled.crate}${pluginTarget.executableExtension}`,
  );
  if (!(await isFile(builtEntryPath))) {
    fail(`cargo built ${bundled.id}, but its binary was not found at ${builtEntryPath}`);
  }
  const stagedEntryPath = join(pluginDirectory, entryName);
  await mkdir(dirname(stagedEntryPath), { recursive: true });
  await cp(builtEntryPath, stagedEntryPath);
  if (pluginTarget.executableExtension === '') {
    await chmod(stagedEntryPath, 0o755);
  }
  if (bundled.uiEntry) {
    await stageFirstPartyUi(bundled, packageDirectory, pluginDirectory);
  }
  await writeStagedManifest(manifestPath, pluginDirectory, entryName);
}

/** Builds (when missing) and stages an isolated plugin UI. */
async function stageFirstPartyUi(
  bundled: Extract<BundledPlugin, { source: 'first-party' }>,
  packageDirectory: string,
  pluginDirectory: string,
): Promise<void> {
  const uiEntry = bundled.uiEntry;
  if (!uiEntry) return;
  const distDirectory = join(packageDirectory, 'ui', 'dist');
  if (!(await isFile(join(distDirectory, 'index.html')))) {
    const viteConfig = join(packageDirectory, 'ui', 'vite.config.ts');
    if (!(await isFile(viteConfig))) {
      fail(`${bundled.id} declares ui entry ${uiEntry}, but ui/dist/index.html is missing with no vite config to build it`);
    }
    console.log(`Building plugin UI for ${bundled.id}...`);
    run('bun', ['x', 'vite', 'build', '--config', viteConfig]);
  }
  await cp(distDirectory, join(pluginDirectory, dirname(uiEntry)), { recursive: true });
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
  fail(`frontend output is missing ${webIndex}; run bun scripts/task.ts frontend-build first`);
}

const binaryPath = resolve(
  repositoryRoot,
  process.env.RELEASE_BINARY?.trim() || join('target', 'release', platform.binaryName),
);
if (!(await isFile(binaryPath))) {
  fail(`compiled desktop binary is missing ${binaryPath}`);
}

// Packaging may also be invoked outside release CI. Check the tag, manifests,
// Packager config, and actual executable before touching existing staging.
const versionCheck = Bun.spawn([process.execPath, 'scripts/check-version.ts', '--binary'], {
  cwd: repositoryRoot,
  env: { ...process.env, RELEASE_TAG: tag, RELEASE_BINARY: binaryPath },
  stdin: 'ignore',
  stdout: 'inherit',
  stderr: 'inherit',
});
if (await versionCheck.exited !== 0) fail('release version verification failed');

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
// Built-in plugins are official features: build each bundled plugin
// for the release target and stage manifest + executable. A missing
// built-in plugin fails packaging loudly instead of shipping an
// app without it.
const pluginTarget = resolveRustTarget(platform.rustTarget);
for (const bundled of BUNDLED_PLUGINS) {
  const manifestPath =
    bundled.source === 'first-party'
      ? join(repositoryRoot, 'plugins', bundled.package, 'plugin.json')
      : join(repositoryRoot, 'examples', bundled.example, 'plugin.json');
  if (!(await isFile(manifestPath))) {
    fail(`bundled plugin manifest is missing ${manifestPath}`);
  }
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as { id?: unknown };
  if (manifest.id !== bundled.id) {
    fail(`${manifestPath} declares id ${String(manifest.id)}, expected ${bundled.id}`);
  }
  const pluginDirectory = join(bundleDirectory, 'plugins', bundled.id);
  if (bundled.runtime === 'napi-vm') {
    await stageNapiVmPlugin(
      bundled,
      join(repositoryRoot, 'examples', bundled.example),
      pluginDirectory,
      platformValue,
    );
    continue;
  }
  if (bundled.runtime === 'declarative') {
    await stageDeclarativePlugin(bundled, pluginDirectory);
    continue;
  }
  if (bundled.source === 'first-party') {
    await stageFirstPartyPlugin(bundled, pluginDirectory, pluginTarget);
    continue;
  }
  await stageProcessExamplePlugin(bundled, pluginDirectory, pluginTarget);
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
