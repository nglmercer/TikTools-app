import { chmod, cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { externalPluginDirectories, PLUGINS_REPO_ENV, PLUGINS_REPO_NAME, resolvePluginsRepository } from './lib/external-plugins.ts';
import { GATEWAY_PLUGIN_ID, stageGatewayWidgets } from './lib/gateway-widgets.ts';
import { detectHostTarget } from './lib/plugin-targets.ts';

export const repositoryRoot = resolve(import.meta.dir, '..');
const cargoWrapper = join(repositoryRoot, 'scripts', 'cargo-with-linker.mjs');
const developmentPluginRoot = join(repositoryRoot, '.dev-plugins');

type PluginManifest = {
  id?: unknown;
  entry?: unknown;
  runtime?: unknown;
  [key: string]: unknown;
};

type NativeAddonDeclaration = {
  package?: unknown;
  root?: unknown;
};

function run(command: string, args: string[], cwd = repositoryRoot): void {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd,
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

async function copyContent(source: string, target: string, directories: readonly string[]): Promise<void> {
  for (const directory of directories) {
    const sourceDirectory = join(source, directory);
    if (await exists(sourceDirectory)) {
      await cp(sourceDirectory, join(target, directory), { recursive: true });
    }
  }
}

/** Host executable suffix, resolved through the shared target helper so dev and release naming agree. */
function stagedEntryName(entry: string): string {
  const name = basename(entry);
  return detectHostTarget().os === 'windows' && !name.toLowerCase().endsWith('.exe') ? `${entry}.exe` : entry;
}

/**
 * Stage one package from the external plugin repository into
 * `.dev-plugins/<id>`. Layouts:
 *
 * - `declarative`: the manifest (plus assets/dist/locales) only.
 * - `process` with `Cargo.toml`: a cargo package built in place.
 * - `process` with `backend/Cargo.toml` (+ optional `ui/`): the backend
 *   binary at the manifest entry plus the built plugin UI.
 * - `napi-vm`: the compiled TypeScript guest plus staged native addons.
 */
async function stagePlugin(pluginsRepository: string, packageDirectory: string): Promise<boolean> {
  const manifestPath = join(packageDirectory, 'plugin.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as PluginManifest;
  const id = requiredString(manifest.id, 'id', manifestPath);
  const runtime = requiredString(manifest.runtime, 'runtime', manifestPath);
  if (!/^[a-z0-9][a-z0-9._-]{0,63}$/.test(id)) {
    throw new Error(`${manifestPath} has an unsafe plugin id: ${id}`);
  }
  const stagedDirectory = join(developmentPluginRoot, id);
  if (runtime === 'napi-vm') {
    return await stageNapiVm(packageDirectory, manifestPath, manifest, id, stagedDirectory);
  }
  if (runtime !== 'declarative' && runtime !== 'process') {
    console.log(`Skipping ${id}: development staging does not build ${runtime} packages.`);
    return false;
  }

  let stagedManifest: PluginManifest = { ...manifest };
  await rm(stagedDirectory, { recursive: true, force: true });
  await mkdir(stagedDirectory, { recursive: true });
  if (runtime === 'process') {
    const backend = (await exists(join(packageDirectory, 'backend', 'Cargo.toml')))
      ? join(packageDirectory, 'backend')
      : packageDirectory;
    const cargoManifestPath = join(backend, 'Cargo.toml');
    if (!(await exists(cargoManifestPath))) {
      throw new Error(`${manifestPath} is a process plugin without Cargo.toml or backend/Cargo.toml`);
    }
    const binaryName = await cargoBinaryName(cargoManifestPath, manifest, manifestPath, backend !== packageDirectory);
    const targetDirectory = join(pluginsRepository, 'target');
    console.log(`Building development plugin ${id}...`);
    run('node', [cargoWrapper, 'build', '--manifest-path', cargoManifestPath, '--target-dir', targetDirectory]);
    const builtEntryPath = join(targetDirectory, 'debug', stagedEntryName(binaryName));
    if (!(await exists(builtEntryPath))) {
      throw new Error(`Cargo built ${id}, but its binary was not found at ${builtEntryPath}`);
    }
    const entry = stagedEntryName(requiredString(manifest.entry, 'entry', manifestPath).replaceAll('\\', '/'));
    const stagedEntryPath = join(stagedDirectory, entry);
    await mkdir(dirname(stagedEntryPath), { recursive: true });
    await cp(builtEntryPath, stagedEntryPath);
    if (detectHostTarget().os !== 'windows') await chmod(stagedEntryPath, 0o755);
    stagedManifest = { ...manifest, entry };
  }
  await copyContent(packageDirectory, stagedDirectory, ['assets', 'dist', 'locales']);
  await stagePluginUi(packageDirectory, manifestPath, stagedManifest, stagedDirectory);
  if (id === GATEWAY_PLUGIN_ID) {
    // Widget bundles are mandatory gateway content and TikTools-owned.
    await stageGatewayWidgets(repositoryRoot, stagedDirectory);
  }
  await writeFile(join(stagedDirectory, 'plugin.json'), `${JSON.stringify(stagedManifest, null, 2)}\n`, 'utf8');
  console.log(`Staged development plugin ${id}.`);
  return true;
}

/**
 * The cargo binary that implements the manifest entry: a single-package
 * backend crate builds its package-named binary; a plugin-root crate builds
 * the binary named after the manifest entry.
 */
async function cargoBinaryName(
  cargoManifestPath: string,
  manifest: PluginManifest,
  manifestPath: string,
  backendCrate: boolean,
): Promise<string> {
  if (!backendCrate) return basename(requiredString(manifest.entry, 'entry', manifestPath).replaceAll('\\', '/'));
  const packageName = (await readFile(cargoManifestPath, 'utf8')).match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!packageName) throw new Error(`${cargoManifestPath} has no package name`);
  return packageName;
}

async function stagePluginUi(
  packageDirectory: string,
  manifestPath: string,
  manifest: PluginManifest,
  stagedDirectory: string,
): Promise<void> {
  const ui = manifest['ui'] as { entry?: unknown } | undefined;
  const entry = typeof ui?.entry === 'string' ? ui.entry : null;
  if (!entry) return;
  if (!entry.startsWith('ui/') || entry.includes('..') || entry.includes('\\')) {
    throw new Error(`${manifestPath} has an unsafe ui entry: ${entry}`);
  }
  const distDirectory = join(packageDirectory, 'ui', 'dist');
  if (!(await exists(join(distDirectory, 'index.html')))) {
    const viteConfig = join(packageDirectory, 'ui', 'vite.config.ts');
    if (!(await exists(viteConfig))) {
      throw new Error(`${manifestPath} declares ui entry ${entry}, but ui/dist/index.html is missing with no vite config to build it`);
    }
    console.log(`Building plugin UI for ${requiredString(manifest.id, 'id', manifestPath)}...`);
    // The UI toolchain belongs to the plugin repository (`bun install` there).
    run('bun', ['x', 'vite', 'build', '--config', viteConfig], dirname(dirname(packageDirectory)));
  }
  await cp(distDirectory, join(stagedDirectory, dirname(entry)), { recursive: true });
}

/**
 * Stages a napi-vm package: compiles the guest with the repository tsc,
 * stages each declared native package, and copies manifest + dist +
 * node_modules. Native sources prefer a sibling checkout of the native
 * package; when one is missing but the package declares `nativeLibs` with a
 * committed lockfile, the host binaries are fetched from their pinned
 * provider instead. Anything else skips the package with a warning.
 */
async function stageNapiVm(
  packageDirectory: string,
  manifestPath: string,
  manifest: PluginManifest,
  id: string,
  stagedDirectory: string,
): Promise<boolean> {
  console.log(`Building development plugin ${id}...`);
  const tsconfigPath = join(packageDirectory, 'tsconfig.json');
  if (!(await exists(tsconfigPath))) {
    throw new Error(`${manifestPath} is napi-vm but has no tsconfig.json`);
  }
  run('node', [
    join(repositoryRoot, 'node_modules', 'typescript', 'bin', 'tsc'),
    '-p',
    tsconfigPath,
    // Resolve ambient types from TikTools when the plugin checkout has no
    // node_modules of its own.
    '--typeRoots',
    join(repositoryRoot, 'node_modules', '@types'),
  ]);

  const nativeAddons = Array.isArray(manifest['nativeAddons'])
    ? (manifest['nativeAddons'] as NativeAddonDeclaration[])
    : [];
  if (nativeAddons.length > 0) {
    run('node', [cargoWrapper, 'build', '-p', 'tiktools-plugin-devkit', '--bin', 'tiktools-plugin-stage-native']);
    const stageBinary =
      join(repositoryRoot, 'target', 'debug', 'tiktools-plugin-stage-native') +
      (process.platform === 'win32' ? '.exe' : '');
    const lockfilePath = join(packageDirectory, 'native-libs.lock.json');
    const canFetch =
      Array.isArray(manifest['nativeLibs']) &&
      (manifest['nativeLibs'] as unknown[]).length > 0 &&
      (await exists(lockfilePath));
    const addons = nativeAddons.map((declaration) => ({
      packageName: requiredString(declaration.package, 'nativeAddons[].package', manifestPath),
      root: requiredString(declaration.root, 'nativeAddons[].root', manifestPath),
    }));
    const siblingOf = (name: string) => join(repositoryRoot, '..', name);
    let missingSibling: string | null = null;
    for (const addon of addons) {
      if (!(await exists(join(siblingOf(addon.packageName), 'package.json')))) {
        missingSibling = addon.packageName;
        break;
      }
    }
    if (missingSibling === null) {
      for (const addon of addons) {
        run(stageBinary, [
          '--package', addon.packageName,
          '--source', siblingOf(addon.packageName),
          '--plugin-dir', packageDirectory,
          '--root', addon.root,
          '--overwrite',
        ]);
      }
    } else if (canFetch) {
      run(stageBinary, [
        '--provider', 'all',
        '--manifest', manifestPath,
        '--plugin-dir', packageDirectory,
        '--lockfile', lockfilePath,
        '--overwrite',
      ]);
    } else {
      console.log(
        `Skipping ${id}: native package source is missing ${siblingOf(missingSibling)} (clone ${missingSibling} beside TikTools, or declare nativeLibs with native-libs.lock.json to fetch from).`,
      );
      return false;
    }
  }

  await rm(stagedDirectory, { recursive: true, force: true });
  await mkdir(stagedDirectory, { recursive: true });
  await writeFile(join(stagedDirectory, 'plugin.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  await copyContent(packageDirectory, stagedDirectory, ['assets', 'dist', 'locales', 'node_modules']);
  return true;
}

/**
 * Stage development plugins from the optional external plugin checkout.
 * A missing checkout, or one package that fails to build, never blocks
 * TikTools startup: it is reported and skipped.
 */
export async function prepareDevelopmentPlugins(): Promise<string> {
  await mkdir(developmentPluginRoot, { recursive: true });
  const pluginsRepository = await resolvePluginsRepository(repositoryRoot);
  if (!pluginsRepository) {
    console.log(
      `No external plugin checkout found; starting without development plugins. ` +
        `Clone ${PLUGINS_REPO_NAME} beside TikTools or set ${PLUGINS_REPO_ENV} to stage them.`,
    );
    return developmentPluginRoot;
  }
  let prepared = 0;
  for (const directory of await externalPluginDirectories(pluginsRepository)) {
    try {
      if (await stagePlugin(pluginsRepository, directory)) prepared += 1;
    } catch (error) {
      console.warn(`Skipping ${basename(directory)}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  console.log(
    `Prepared ${prepared} development plugin${prepared === 1 ? '' : 's'} from ${pluginsRepository} in ${developmentPluginRoot}`,
  );
  return developmentPluginRoot;
}
