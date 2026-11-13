import { chmod, cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { externalPluginDirectories, PLUGINS_REPO_ENV, PLUGINS_REPO_NAME, resolvePluginsRepository } from './lib/external-plugins.ts';
import { compileNapiVmGuest, prepareNativeAddons, siblingSourceDirectory } from './lib/napi-vm.ts';
import { assertPluginId } from './lib/plugin-id.ts';
import { GATEWAY_PLUGIN_ID, stageGatewayWidgets } from './lib/gateway-widgets.ts';
import { detectHostTarget } from './lib/plugin-targets.ts';
import { buildPluginUi, executableName, pluginCargoManifest, processBinaryName } from './lib/plugin-build.ts';

export const repositoryRoot = resolve(import.meta.dir, '..');
const cargoWrapper = join(repositoryRoot, 'scripts', 'cargo-with-linker.mjs');
const developmentPluginRoot = join(repositoryRoot, '.dev-plugins');

type PluginManifest = {
  id?: unknown;
  entry?: unknown;
  runtime?: unknown;
  [key: string]: unknown;
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
  return detectHostTarget().os === 'windows' && !entry.toLowerCase().endsWith('.exe') ? `${entry}.exe` : entry;
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
  const id = assertPluginId(manifest.id, manifestPath);
  const runtime = requiredString(manifest.runtime, 'runtime', manifestPath);
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
    const cargoManifestPath = await pluginCargoManifest(packageDirectory);
    if (!cargoManifestPath) {
      throw new Error(`${manifestPath} is a process plugin without Cargo.toml or backend/Cargo.toml`);
    }
    const binaryName = await processBinaryName(cargoManifestPath, requiredString(manifest.entry, 'entry', manifestPath));
    const targetDirectory = join(pluginsRepository, 'target');
    console.log(`Building development plugin ${id}...`);
    run('node', [cargoWrapper, 'build', '--locked', '--bin', binaryName, '--manifest-path', cargoManifestPath, '--target-dir', targetDirectory]);
    const builtEntryPath = join(targetDirectory, 'debug', executableName(binaryName, detectHostTarget().os === 'windows'));
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

async function stagePluginUi(
  packageDirectory: string,
  manifestPath: string,
  manifest: PluginManifest,
  stagedDirectory: string,
): Promise<void> {
  const ui = await buildPluginUi({ directory: packageDirectory, manifestPath, manifest, exec: run });
  if (ui) await cp(ui.source, join(stagedDirectory, dirname(ui.entry)), { recursive: true });
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
  await compileNapiVmGuest({ repositoryRoot, packageDirectory, manifestPath, exec: (command, args) => run(command, args) });
  const host = detectHostTarget();
  const native = await prepareNativeAddons({
    repositoryRoot,
    packageDirectory,
    manifestPath,
    manifest,
    host,
    target: host,
    profile: 'debug',
    cargo: (args) => run('node', [cargoWrapper, ...args]),
    exec: (command, args) => run(command, args),
  });
  if (native.status === 'unavailable') {
    // Development never fails startup over optional native sources: skip the package.
    const missing = native.missing[0] ?? 'the native package';
    console.log(
      `Skipping ${id}: native package source is missing ${siblingSourceDirectory(repositoryRoot, missing)} (clone ${missing} beside TikTools, or declare nativeLibs with native-libs.lock.json to fetch from).`,
    );
    return false;
  }

  await rm(stagedDirectory, { recursive: true, force: true });
  await mkdir(stagedDirectory, { recursive: true });
  await copyContent(packageDirectory, stagedDirectory, ['assets', 'dist', 'locales', 'node_modules']);
  await writeFile(join(stagedDirectory, 'plugin.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  return true;
}

/**
 * Stage development plugins from the optional external plugin checkout.
 * A missing checkout, or one package that fails to build, never blocks
 * TikTools startup: it is reported and skipped.
 */
export async function prepareDevelopmentPlugins(): Promise<string> {
  // This root contains only generated packages. Never load removed or failed
  // packages left by an earlier checkout, including when staging is disabled.
  await rm(developmentPluginRoot, { recursive: true, force: true });
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
