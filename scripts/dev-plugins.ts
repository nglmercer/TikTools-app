import { chmod, cp, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { ensureGatewayWidgetsStaged } from './lib/gateway-widgets.ts';
import { detectHostTarget } from './lib/plugin-targets.ts';

export const repositoryRoot = resolve(import.meta.dir, '..');
const cargoWrapper = join(repositoryRoot, 'scripts', 'cargo-with-linker.mjs');
const examplesRoot = join(repositoryRoot, 'examples');
const firstPartyRoot = join(repositoryRoot, 'plugins');
const developmentPluginRoot = join(repositoryRoot, '.dev-plugins');

type ExampleManifest = {
  id?: unknown;
  entry?: unknown;
  runtime?: unknown;
  [key: string]: unknown;
};

function run(command: string, args: string[]): void {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd: repositoryRoot,
    stdout: 'inherit',
    stderr: 'inherit',
  });
  if (!result.success) {
    throw new Error(`${command} ${args.join(' ')} exited with code ${result.exitCode}`);
  }
}

function requiredString(value: unknown, field: string, file: string): string {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`${file} has no valid ${field}`);
  }
  return value;
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function stageExample(exampleDirectory: string): Promise<boolean> {
  const manifestPath = join(exampleDirectory, 'plugin.json');
  const cargoManifestPath = join(exampleDirectory, 'Cargo.toml');
  if (!(await exists(manifestPath))) {
    return false;
  }

  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as ExampleManifest;
  const id = requiredString(manifest.id, 'id', manifestPath);
  const runtime = requiredString(manifest.runtime, 'runtime', manifestPath);
  if (!/^[a-z0-9][a-z0-9._-]{0,63}$/.test(id)) {
    throw new Error(`${manifestPath} has an unsafe plugin id: ${id}`);
  }
  // Declarative packages ship no executable entry: the manifest alone is
  // staged, with no cargo build.
  if (runtime === 'declarative') {
    if (await exists(cargoManifestPath)) {
      throw new Error(`${manifestPath} is declarative and must not ship a Cargo.toml`);
    }
    console.log(`Staging declarative development plugin ${id}...`);
    const packageDirectory = join(developmentPluginRoot, id);
    await rm(packageDirectory, { recursive: true, force: true });
    await mkdir(packageDirectory, { recursive: true });
    await writeFile(join(packageDirectory, 'plugin.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
    for (const directory of ['assets', 'dist', 'locales']) {
      const sourceDirectory = join(exampleDirectory, directory);
      if (await exists(sourceDirectory)) {
        await cp(sourceDirectory, join(packageDirectory, directory), { recursive: true });
      }
    }
    return true;
  }
  if (!(await exists(cargoManifestPath))) {
    return false;
  }
  const sourceEntry = requiredString(manifest.entry, 'entry', manifestPath).replaceAll('\\', '/');
  if (runtime !== 'process') {
    console.log(`Skipping ${id}: development bootstrap only builds process examples.`);
    return false;
  }

  console.log(`Building development plugin ${id}...`);
  run('node', [cargoWrapper, 'build', '--manifest-path', cargoManifestPath]);
  if (id === 'tiktools.event-gateway') {
    // Widget bundles are mandatory gateway content: a fresh clone must
    // stage them with no manual sync step.
    await ensureGatewayWidgetsStaged(repositoryRoot);
  }

  // Development builds are host-only, but resolve the executable suffix
  // through the same shared target helper so dev and release naming agree.
  const hostTarget = detectHostTarget();
  const sourceEntryName = basename(sourceEntry);
  const stagedEntry =
    hostTarget.os === 'windows' && !sourceEntryName.toLowerCase().endsWith('.exe')
      ? `${sourceEntry}.exe`
      : sourceEntry;
  const builtEntryPath = join(exampleDirectory, 'target', 'debug', stagedEntry);
  if (!(await exists(builtEntryPath))) {
    throw new Error(`Cargo built ${id}, but its declared entry was not found at ${builtEntryPath}`);
  }

  const packageDirectory = join(developmentPluginRoot, id);
  const stagedEntryPath = join(packageDirectory, stagedEntry);
  await rm(packageDirectory, { recursive: true, force: true });
  await mkdir(dirname(stagedEntryPath), { recursive: true });
  await cp(builtEntryPath, stagedEntryPath);
  if (hostTarget.os !== 'windows') {
    await chmod(stagedEntryPath, 0o755);
  }

  await writeFile(
    join(packageDirectory, 'plugin.json'),
    `${JSON.stringify({ ...manifest, entry: stagedEntry }, null, 2)}\n`,
    'utf8',
  );

  for (const directory of ['assets', 'dist', 'locales']) {
    const sourceDirectory = join(exampleDirectory, directory);
    if (await exists(sourceDirectory)) {
      await cp(sourceDirectory, join(packageDirectory, directory), { recursive: true });
    }
  }
  return true;
}

export async function prepareDevelopmentPlugins(): Promise<string> {
  await mkdir(developmentPluginRoot, { recursive: true });
  let prepared = 0;
  for (const entry of await readdir(examplesRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    if (await stageExample(join(examplesRoot, entry.name))) prepared += 1;
  }
  if (await exists(firstPartyRoot)) {
    for (const entry of await readdir(firstPartyRoot, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (await stageFirstParty(join(firstPartyRoot, entry.name))) prepared += 1;
    }
  }
  console.log(
    `Prepared ${prepared} development plugin${prepared === 1 ? '' : 's'} in ${developmentPluginRoot}`,
  );
  return developmentPluginRoot;
}

/**
 * Stages a first-party package from `plugins/` (manifest + compiled
 * backend at the manifest entry + built UI dist). Unlike examples, the
 * backend is a workspace member built with `cargo build -p`, and the UI
 * is built with its own vite config when `ui/dist/` is absent.
 */
async function stageFirstParty(packageDirectory: string): Promise<boolean> {
  const manifestPath = join(packageDirectory, 'plugin.json');
  if (!(await exists(manifestPath))) {
    return false;
  }
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as ExampleManifest;
  const id = requiredString(manifest.id, 'id', manifestPath);
  if (!/^[a-z0-9][a-z0-9._-]{0,63}$/.test(id)) {
    throw new Error(`${manifestPath} has an unsafe plugin id: ${id}`);
  }
  const runtime = requiredString(manifest.runtime, 'runtime', manifestPath);
  const stagedDirectory = join(developmentPluginRoot, id);
  await rm(stagedDirectory, { recursive: true, force: true });
  await mkdir(stagedDirectory, { recursive: true });

  let stagedManifest: ExampleManifest = { ...manifest };
  if (runtime === 'process') {
    stagedManifest = await stageFirstPartyBackend(packageDirectory, manifestPath, manifest, stagedDirectory);
  } else if (runtime !== 'declarative') {
    console.log(`Skipping ${id}: development bootstrap only builds process packages.`);
    return false;
  }
  await stageFirstPartyUi(packageDirectory, manifestPath, stagedManifest, stagedDirectory);

  await writeFile(
    join(stagedDirectory, 'plugin.json'),
    `${JSON.stringify(stagedManifest, null, 2)}\n`,
    'utf8',
  );
  console.log(`Staged first-party development plugin ${id}.`);
  return true;
}

async function stageFirstPartyBackend(
  packageDirectory: string,
  manifestPath: string,
  manifest: ExampleManifest,
  stagedDirectory: string,
): Promise<ExampleManifest> {
  const id = requiredString(manifest.id, 'id', manifestPath);
  const cargoManifestPath = join(packageDirectory, 'backend', 'Cargo.toml');
  if (!(await exists(cargoManifestPath))) {
    throw new Error(`${manifestPath} is a process plugin without backend/Cargo.toml`);
  }
  const cargoManifest = await readFile(cargoManifestPath, 'utf8');
  const packageName = cargoManifest.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!packageName) {
    throw new Error(`${cargoManifestPath} has no package name`);
  }
  console.log(`Building first-party development plugin ${id}...`);
  run('node', [cargoWrapper, 'build', '-p', packageName]);

  const hostTarget = detectHostTarget();
  const sourceEntry = requiredString(manifest.entry, 'entry', manifestPath).replaceAll('\\', '/');
  const sourceEntryName = basename(sourceEntry);
  const stagedEntry =
    hostTarget.os === 'windows' && !sourceEntryName.toLowerCase().endsWith('.exe')
      ? `${sourceEntry}.exe`
      : sourceEntry;
  const binaryName = hostTarget.os === 'windows' ? `${packageName}.exe` : packageName;
  const candidates = [
    join(repositoryRoot, 'target', 'debug', binaryName),
    join(packageDirectory, 'backend', 'target', 'debug', binaryName),
  ];
  const builtEntryPath = candidates.find((candidate) => Bun.file(candidate).size > 0);
  if (!builtEntryPath) {
    throw new Error(
      `Cargo built ${packageName}, but its binary was not found at ${candidates.join(' or ')}`,
    );
  }
  const stagedEntryPath = join(stagedDirectory, stagedEntry);
  await mkdir(dirname(stagedEntryPath), { recursive: true });
  await cp(builtEntryPath, stagedEntryPath);
  if (hostTarget.os !== 'windows') {
    await chmod(stagedEntryPath, 0o755);
  }
  return { ...manifest, entry: stagedEntry };
}

async function stageFirstPartyUi(
  packageDirectory: string,
  manifestPath: string,
  manifest: ExampleManifest,
  stagedDirectory: string,
): Promise<void> {
  const ui = manifest['ui'] as { entry?: unknown } | undefined;
  const entry = typeof ui?.entry === 'string' ? ui.entry : null;
  if (!entry) {
    return;
  }
  if (!entry.startsWith('ui/') || entry.includes('..') || entry.includes('\\')) {
    throw new Error(`${manifestPath} has an unsafe ui entry: ${entry}`);
  }
  const distDirectory = join(packageDirectory, 'ui', 'dist');
  if (!(await exists(join(distDirectory, 'index.html')))) {
    const viteConfig = join(packageDirectory, 'ui', 'vite.config.ts');
    if (!(await exists(viteConfig))) {
      throw new Error(
        `${manifestPath} declares ui entry ${entry}, but ui/dist/index.html is missing with no vite config to build it`,
      );
    }
    console.log(`Building plugin UI for ${requiredString(manifest.id, 'id', manifestPath)}...`);
    run('bun', ['x', 'vite', 'build', '--config', viteConfig]);
  }
  await cp(distDirectory, join(stagedDirectory, dirname(entry)), { recursive: true });
}
