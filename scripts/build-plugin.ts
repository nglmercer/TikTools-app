import { mkdir, readFile, stat } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { PLUGINS_REPO_ENV, PLUGINS_REPO_NAME, externalPluginDirectories, resolvePluginsRepository } from './lib/external-plugins.ts';
import { compileNapiVmGuest, prepareNativeAddons } from './lib/napi-vm.ts';
import { assertPluginId } from './lib/plugin-id.ts';
import { GATEWAY_PLUGIN_ID, GATEWAY_WIDGET_KINDS, stageGatewayWidgets } from './lib/gateway-widgets.ts';
import {
  artifactFileName,
  detectHostTarget,
  resolveRustTarget,
  supportedRustTargets,
  validatePluginVersion,
  type PluginBuildTarget,
} from './lib/plugin-targets.ts';

const repositoryRoot = resolve(import.meta.dir, '..');
const cargoWrapper = join(repositoryRoot, 'scripts', 'cargo-with-linker.mjs');
const defaultOutDirectory = join(repositoryRoot, 'dist', 'plugins');

type ExampleManifest = {
  id?: unknown;
  entry?: unknown;
  runtime?: unknown;
  version?: unknown;
  [key: string]: unknown;
};

function fail(message: string): never {
  throw new Error(`Plugin packaging failed: ${message}`);
}

function runInherit(command: string, args: string[]): void {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd: repositoryRoot,
    stdout: 'inherit',
    stderr: 'inherit',
  });
  if (!result.success) fail(`${command} ${args.join(' ')} exited with code ${result.exitCode}`);
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

function printHelp(): void {
  console.log(`Build distributable .plugin archives from the external plugin checkout
(${PLUGINS_REPO_ENV}, or a sibling ${PLUGINS_REPO_NAME}/).

Usage:
  bun scripts/task.ts plugins --plugin <name>
  bun scripts/task.ts plugins --plugin <name> --target <rust-target>
  bun scripts/task.ts plugins --all
  bun scripts/task.ts plugins --all --target <rust-target>
  bun scripts/task.ts plugins --all
  bun scripts/task.ts plugins --all --target <rust-target>

Options:
  --plugin <name>   Plugin directory or manifest id (repeatable)
  --all             Build every plugin with a plugin.json
  --target <triple> Rust target triple (default: host target)
  --host            Build for the host target explicitly
  --out <dir>       Output directory (default: dist/plugins)
  --debug           Use the debug cargo profile (default: release)
  --sign <key>      Sign archives with an enrolled publisher private key
  --help            Show this message

Supported targets:
  ${supportedRustTargets().join(', ')}

Output:
  <out>/<plugin-id>-<version>-<plugin-target>.plugin (target-independent WASM and declarative plugins: <id>-<version>.plugin)`);
}

const rawArgs = process.argv.slice(2);
const selected: string[] = [];
let buildAll = false;
let outDirectory = process.env.PLUGIN_OUT_DIR?.trim() || defaultOutDirectory;
let profile: 'release' | 'debug' = 'release';
let requestedTarget: string | null = null;
let explicitHost = false;
let signingKey: string | null = null;

for (let index = 0; index < rawArgs.length; index += 1) {
  const argument = rawArgs[index];
  if (argument === '--help' || argument === '-h') {
    printHelp();
    process.exit(0);
  } else if (argument === '--all') {
    buildAll = true;
  } else if (argument === '--debug') {
    profile = 'debug';
  } else if (argument === '--host') {
    explicitHost = true;
  } else if (argument === '--sign') {
    const value = rawArgs[index + 1];
    if (!value || value.startsWith('--')) fail('--sign requires a key path');
    signingKey = resolve(repositoryRoot, value);
    index += 1;
  } else if (argument === '--target') {
    const value = rawArgs[index + 1];
    if (!value) fail('--target requires a value');
    requestedTarget = value;
    index += 1;
  } else if (argument.startsWith('--target=')) {
    requestedTarget = argument.slice('--target='.length);
  } else if (argument === '--plugin' || argument === '--example') {
    const value = rawArgs[index + 1];
    if (!value) fail(`${argument} requires a value`);
    selected.push(value);
    index += 1;
  } else if (argument.startsWith('--plugin=')) {
    selected.push(argument.slice('--plugin='.length));
  } else if (argument.startsWith('--out=')) {
    outDirectory = argument.slice('--out='.length);
  } else if (argument === '--out' || argument === '--outDir') {
    const value = rawArgs[index + 1];
    if (!value) fail(`${argument} requires a value`);
    outDirectory = value;
    index += 1;
  } else if (!argument.startsWith('--')) {
    selected.push(argument);
  } else {
    fail(`unknown argument: ${argument} (see --help)`);
  }
}

if (!requestedTarget?.trim() && !explicitHost) {
  // Default: host target.
} else if (explicitHost && requestedTarget?.trim()) {
  fail('--host and --target are mutually exclusive');
}

let selectedTarget: PluginBuildTarget;
try {
  selectedTarget = requestedTarget?.trim()
    ? resolveRustTarget(requestedTarget.trim())
    : detectHostTarget();
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
  throw error;
}

if (!buildAll && selected.length === 0) {
  printHelp();
  fail('expected --all or --plugin <name>');
}
if (!outDirectory.trim()) fail('output directory must not be empty');
outDirectory = resolve(repositoryRoot, outDirectory);

/// Compiles a napi-vm example guest with the repository tsc, stages
/// each declared native package, and returns the built entry path for
/// the packager. Staging prefers a sibling source checkout on host
/// builds (fast, offline, local iteration); otherwise a declared
/// `nativeLibs` entry plus its committed lockfile is fetched from its
/// pinned provider — which also unlocks cross-target builds, since the
/// provider serves every target's binary from one manifest. Packaging
/// fails when the required native components cannot be provided.
async function buildNapiVmExample(
  directory: string,
  manifestPath: string,
  manifest: ExampleManifest,
  sourceEntry: string,
): Promise<string> {
  const exec = (command: string, args: string[]) => runInherit(command, args);
  try {
    await compileNapiVmGuest({ repositoryRoot, packageDirectory: directory, manifestPath, exec });
    const host = detectHostTarget();
    const native = await prepareNativeAddons({
      repositoryRoot,
      packageDirectory: directory,
      manifestPath,
      manifest,
      host,
      target: selectedTarget,
      profile,
      cargo: (args) => runInherit('node', [cargoWrapper, ...args]),
      exec,
    });
    if (native.status === 'unavailable') {
      throw new Error(
        native.reason === 'cross-target'
          ? `${manifestPath} is napi-vm: cross-target builds need per-target native binaries (${selectedTarget.rustTarget} requested on a ${host.rustTarget} host); declare nativeLibs in the manifest and pin native-libs.lock.json with stage-native --pin`
          : `${manifestPath} needs the ${native.missing.join(', ')} source checkout beside the repository, or a nativeLibs declaration with native-libs.lock.json to fetch from`,
      );
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    // `runInherit` failures already carry the packaging prefix.
    if (message.startsWith('Plugin packaging failed:')) throw error;
    fail(message);
  }
  const builtEntryPath = join(directory, sourceEntry);
  if (!(await exists(builtEntryPath))) {
    fail(`tsc built ${manifest.id}, but its declared entry was not found at ${builtEntryPath}`);
  }
  return builtEntryPath;
}

type DiscoveredExample = { directory: string; manifestPath: string; manifest: ExampleManifest };

const pluginsRepository = await resolvePluginsRepository(repositoryRoot);
if (!pluginsRepository) {
  fail(`no plugin sources: clone ${PLUGINS_REPO_NAME} beside TikTools or set ${PLUGINS_REPO_ENV}`);
}
const discovered: DiscoveredExample[] = [];
for (const directory of await externalPluginDirectories(pluginsRepository)) {
  const manifestPath = join(directory, 'plugin.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as ExampleManifest;
  // napi-vm examples ship TypeScript; declarative examples need no build.
  // Compiled runtimes need a Cargo project.
  if (manifest.runtime !== 'napi-vm' && manifest.runtime !== 'declarative' && !(await exists(join(directory, 'Cargo.toml')))) continue;
  discovered.push({ directory, manifestPath, manifest });
}

const targets = buildAll
  ? discovered
  : selected.map((selector) => {
      const found = discovered.find(
        ({ directory, manifest }) =>
          basename(directory) === selector || manifest.id === selector,
      );
      if (!found) {
        const known = discovered
          .map(({ directory, manifest }) => `${basename(directory)} (${String(manifest.id)})`)
          .join(', ');
        fail(`unknown plugin ${selector}; known: ${known || 'none'}`);
      }
      return found;
    });

if (targets.length === 0) fail(`no buildable plugins found under ${pluginsRepository}/plugins`);

await mkdir(outDirectory, { recursive: true });

const VALID_RUNTIMES = new Set(['native', 'process', 'wasm', 'napi-vm', 'declarative']);

const built: string[] = [];
for (const { directory, manifestPath, manifest } of targets) {
  const id = assertPluginId(manifest.id, `Plugin packaging failed: ${manifestPath}`);
  const version = validatePluginVersion(manifest.version, manifestPath);
  if (signingKey && (typeof manifest.publisher !== 'string' || !manifest.publisher)) {
    fail(`${manifestPath} needs a publisher when signing`);
  }
  const sourceEntryRaw = manifest.runtime === 'declarative' ? '' :
    typeof manifest.entry === 'string' && manifest.entry.trim().length > 0
      ? manifest.entry
      : fail(`${manifestPath} has no valid entry`);
  const sourceEntry = (sourceEntryRaw as string).replaceAll('\\', '/');
  const runtimeRaw =
    typeof manifest.runtime === 'string' && manifest.runtime.trim().length > 0
      ? (manifest.runtime as string)
      : fail(`${manifestPath} has no valid runtime`);
  const runtime = runtimeRaw.trim();
  if (!VALID_RUNTIMES.has(runtime)) {
    fail(`${manifestPath} has an unsupported runtime: ${runtime}`);
  }
  if (sourceEntry.includes('\0') || sourceEntry.startsWith('/') || sourceEntry.split('/').includes('..')) {
    fail(`${manifestPath} has an unsafe entry: ${sourceEntry}`);
  }
  if (id === GATEWAY_PLUGIN_ID) {
    // Widget bundles are mandatory gateway content and TikTools-owned:
    // build and stage them into the package's (gitignored) dist/ before the
    // packager snapshots it, and fail loudly when missing.
    await stageGatewayWidgets(repositoryRoot, directory);
  }

  // Declarative and WASM packages are target-independent;
  // compiled native/process entries are platform-specific, as are napi-vm
  // archives (each carries its target's staged `.node` binaries).
  const packageTarget = runtime === 'wasm' || runtime === 'declarative' ? null : selectedTarget.pluginTarget;

  console.log(
    `Building plugin ${id} (${profile}, ${packageTarget ?? 'target-independent'})...`,
  );
  let builtEntryPath: string | null = null;
  if (runtime === 'declarative') {
    // Declarative packages are interpreted by the host and have no executable.
  } else if (runtime === 'napi-vm') {
    builtEntryPath = await buildNapiVmExample(directory, manifestPath, manifest, sourceEntry);
  } else {
    runInherit('node', [
      cargoWrapper,
      'build',
      ...(profile === 'release' ? ['--release'] : []),
      '--target',
      selectedTarget.rustTarget,
      '--manifest-path',
      join(directory, 'Cargo.toml'),
      '--target-dir',
      join(pluginsRepository, 'target'),
    ]);

    // The executable suffix derives from the requested build target, never
    // from process.platform, so explicit cross-target builds resolve the
    // correct Cargo output (e.g. demo.exe on a Linux host).
    const builtEntryName =
      selectedTarget.os === 'windows' && !basename(sourceEntry).toLowerCase().endsWith('.exe')
        ? `${sourceEntry}.exe`
        : sourceEntry;
    builtEntryPath = join(
      pluginsRepository,
      'target',
      selectedTarget.rustTarget,
      profile,
      builtEntryName,
    );
    if (!(await exists(builtEntryPath))) {
      fail(`cargo built ${id}, but its declared entry was not found at ${builtEntryPath}`);
    }
  }

  const archiveName = artifactFileName(id, version, packageTarget);
  const archivePath = join(outDirectory, archiveName);
  runInherit('node', [
    cargoWrapper,
    'run',
    '-p',
    'tiktools-plugin-devkit',
    '--bin',
    'tiktools-plugin-pack',
    '--',
    '--manifest',
    manifestPath,
    ...(builtEntryPath ? ['--entry', builtEntryPath] : []),
    '--output',
    archivePath,
    ...(packageTarget ? ['--target', packageTarget] : []),
    ...(signingKey ? ['--sign', signingKey] : []),
  ]);

  // Post-package validation: archive exists and packaged manifest declares
  // the requested target.
  if (!(await exists(archivePath))) {
    fail(`packager did not produce ${archivePath}`);
  }
  await verifyPackagedManifest(archivePath, id, version, manifest.publisher, packageTarget, runtime === 'declarative');
  if (signingKey) {
    const signature = JSON.parse(await readArchiveFile(archivePath, `${id}/signature.json`)) as { publisher?: unknown };
    if (signature.publisher !== manifest.publisher) {
      fail(`archive ${archivePath} signing key publisher does not match its manifest`);
    }
  }
  if (id === GATEWAY_PLUGIN_ID) {
    await verifyPackagedWidgets(archivePath, id);
  }

  console.log(`Created ${basename(archivePath)}`);
  built.push(archivePath);
}

async function verifyPackagedManifest(
  archivePath: string,
  id: string,
  version: string,
  publisher: unknown,
  packageTarget: string | null,
  declarative: boolean,
): Promise<void> {
  const entries = await readArchiveEntries(archivePath);
  const manifestEntry = entries.find((entry) => entry.endsWith('/plugin.json'));
  if (!manifestEntry) {
    fail(`archive ${archivePath} does not contain plugin.json`);
  }
  const manifestText = await readArchiveFile(archivePath, manifestEntry as string);
  const packaged = JSON.parse(manifestText) as { id?: unknown; version?: unknown; publisher?: unknown; targets?: unknown; entry?: unknown };
  if (packaged.id !== id || packaged.version !== version || packaged.publisher !== publisher) {
    fail(`archive ${archivePath} has mismatched id, version, or publisher`);
  }
  if (packageTarget) {
    if (!Array.isArray(packaged.targets) || packaged.targets[0] !== packageTarget) {
      fail(`archive ${archivePath} first packaged target is not ${packageTarget}`);
    }
    if (typeof packaged.entry !== 'string' || packaged.entry.trim().length === 0) {
      fail(`archive ${archivePath} has no valid packaged entry`);
    }
  } else if (Array.isArray(packaged.targets) && (packaged.targets as unknown[]).length > 0) {
    fail(`archive ${archivePath} should be target-independent but declares targets`);
  }
  const entryFile = entries.find(
    (entry) => entry === `${id}/${String(packaged.entry).replaceAll('\\', '/')}`,
  );
  if (!declarative && !entryFile) {
    fail(`archive ${archivePath} does not contain declared entry ${String(packaged.entry)}`);
  }
}

async function verifyPackagedWidgets(archivePath: string, id: string): Promise<void> {
  // Never silently ship a gateway whose /widgets/* routes 404: every
  // staged bundle must be inside the archive we just wrote.
  const entries = await readArchiveEntries(archivePath);
  for (const kind of GATEWAY_WIDGET_KINDS) {
    const entry = `${id}/dist/widgets/${kind}/index.html`;
    if (!entries.includes(entry)) {
      fail(`archive ${archivePath} is missing ${entry}; refusing to ship a gateway without widget assets`);
    }
  }
}

async function readArchiveEntries(archivePath: string): Promise<string[]> {
  const file = Bun.file(archivePath);
  const buffer = await file.arrayBuffer();
  const { entries } = parseZipCentralDirectory(new Uint8Array(buffer));
  return entries;
}

async function readArchiveFile(archivePath: string, entryName: string): Promise<string> {
  const file = Bun.file(archivePath);
  const bytes = new Uint8Array(await file.arrayBuffer());
  return extractZipEntry(bytes, entryName);
}

// Minimal ZIP reader (stored + deflate) so build validation does not need
// extra dependencies. Archives are written by tiktools-plugin-pack.
function parseZipCentralDirectory(bytes: Uint8Array): { entries: string[] } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let eocd = -1;
  const start = Math.max(0, bytes.length - 65557);
  for (let i = bytes.length - 22; i >= start; i -= 1) {
    if (
      bytes[i] === 0x50 &&
      bytes[i + 1] === 0x4b &&
      bytes[i + 2] === 0x05 &&
      bytes[i + 3] === 0x06
    ) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) fail('could not parse plugin archive directory');
  const count = view.getUint16(eocd + 10, true);
  const centralOffset = view.getUint32(eocd + 16, true);
  const entries: string[] = [];
  let offset = centralOffset;
  const decoder = new TextDecoder();
  for (let n = 0; n < count; n += 1) {
    if (view.getUint32(offset, true) !== 0x02014b50) fail('invalid plugin archive directory');
    const nameLen = view.getUint16(offset + 28, true);
    const extraLen = view.getUint16(offset + 30, true);
    const commentLen = view.getUint16(offset + 32, true);
    const nameBytes = bytes.slice(offset + 46, offset + 46 + nameLen);
    entries.push(decoder.decode(nameBytes));
    offset += 46 + nameLen + extraLen + commentLen;
  }
  return { entries };
}

function extractZipEntry(bytes: Uint8Array, entryName: string): string {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  const target = encoder.encode(entryName);
  let offset = 0;
  while (offset + 30 <= bytes.length) {
    const sig = view.getUint32(offset, true);
    if (sig === 0x02014b50 || sig === 0x06054b50) break;
    if (sig !== 0x04034b50) fail('invalid plugin archive entry');
    const method = view.getUint16(offset + 8, true);
    const compressedSize = view.getUint32(offset + 18, true);
    const nameLen = view.getUint16(offset + 26, true);
    const extraLen = view.getUint16(offset + 28, true);
    const nameBytes = bytes.slice(offset + 30, offset + 30 + nameLen);
    const dataStart = offset + 30 + nameLen + extraLen;
    const dataEnd = dataStart + compressedSize;
    if (dataEnd > bytes.length) fail('truncated plugin archive entry');
    if (nameBytes.length === target.length && nameBytes.every((b, i) => b === target[i as number])) {
      const data = bytes.slice(dataStart, dataEnd);
      if (method === 0) return decoder.decode(data);
      if (method === 8) return decoder.decode(Bun.inflateSync(data));
      fail(`unsupported compression method ${method} in plugin archive`);
    }
    offset = dataEnd;
  }
  fail(`archive does not contain ${entryName}`);
  throw new Error('unreachable');
}

console.log(`Built ${built.length} plugin${built.length === 1 ? '' : 's'} in ${outDirectory}`);
