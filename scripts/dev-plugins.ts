import { chmod, cp, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { detectHostTarget } from './lib/plugin-targets.ts';

export const repositoryRoot = resolve(import.meta.dir, '..');
const examplesRoot = join(repositoryRoot, 'examples');
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
  if (!(await exists(manifestPath)) || !(await exists(cargoManifestPath))) {
    return false;
  }

  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as ExampleManifest;
  const id = requiredString(manifest.id, 'id', manifestPath);
  const sourceEntry = requiredString(manifest.entry, 'entry', manifestPath).replaceAll('\\', '/');
  const runtime = requiredString(manifest.runtime, 'runtime', manifestPath);
  if (runtime !== 'process') {
    console.log(`Skipping ${id}: development bootstrap only builds process examples.`);
    return false;
  }
  if (!/^[a-z0-9][a-z0-9._-]{0,63}$/.test(id)) {
    throw new Error(`${manifestPath} has an unsafe plugin id: ${id}`);
  }

  console.log(`Building development plugin ${id}...`);
  run('cargo', ['build', '--manifest-path', cargoManifestPath]);

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
  console.log(
    `Prepared ${prepared} development plugin${prepared === 1 ? '' : 's'} in ${developmentPluginRoot}`,
  );
  return developmentPluginRoot;
}

