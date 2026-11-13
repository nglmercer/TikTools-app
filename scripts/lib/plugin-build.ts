import { readFile, stat } from 'node:fs/promises';
import { basename, join } from 'node:path';

export async function fileExists(path: string): Promise<boolean> {
  return stat(path).then((info) => info.isFile(), () => false);
}

/** Both flat crates and packages with a backend/ crate are supported. */
export async function pluginCargoManifest(directory: string): Promise<string | null> {
  for (const path of [join(directory, 'Cargo.toml'), join(directory, 'backend', 'Cargo.toml')]) {
    if (await fileExists(path)) return path;
  }
  return null;
}

/** Cargo output is flat even when the installed manifest entry is nested. */
export function executableName(entry: string, windows: boolean): string {
  const name = basename(entry.replaceAll('\\', '/'));
  return windows && !name.toLowerCase().endsWith('.exe') ? `${name}.exe` : name;
}

export async function processBinaryName(cargoManifest: string, entry: string): Promise<string> {
  const cargo = Bun.TOML.parse(await readFile(cargoManifest, 'utf8')) as {
    package?: { name?: string };
    bin?: Array<{ name?: string }>;
  };
  const entryName = executableName(entry, false).replace(/\.exe$/i, '');
  const bins = cargo.bin?.flatMap((bin) => bin.name ? [bin.name] : []) ?? [];
  if (bins.includes(entryName)) return entryName;
  if (bins.length === 1) return bins[0]!;
  if (bins.length === 0 && cargo.package?.name) return cargo.package.name;
  throw new Error(`${cargoManifest} has no unambiguous process binary for ${entry}`);
}

/** Rebuild source UIs on every preparation so checked-in dist never hides edits. */
export async function buildPluginUi(options: {
  directory: string;
  manifestPath: string;
  manifest: Record<string, unknown>;
  exec: (command: string, args: string[], cwd: string) => void | Promise<void>;
}): Promise<{ source: string; entry: string } | null> {
  const ui = options.manifest.ui as { entry?: unknown } | undefined;
  if (ui?.entry === undefined) return null;
  const entry = ui.entry;
  if (typeof entry !== 'string' || !entry.startsWith('ui/') || entry.includes('..') || entry.includes('\\') || entry.includes('\0')) {
    throw new Error(`${options.manifestPath} has an unsafe ui entry: ${String(entry)}`);
  }
  const config = join(options.directory, 'ui', 'vite.config.ts');
  if (await fileExists(config)) {
    await options.exec(process.execPath, ['x', '--no-install', 'vite', 'build', '--config', config], join(options.directory, '..', '..'));
  }
  if (!(await fileExists(join(options.directory, entry)))) {
    throw new Error(`${options.manifestPath} declares ui entry ${entry}, but it is missing; install the plugin checkout's dependencies and build its UI`);
  }
  return { source: join(options.directory, 'ui', 'dist'), entry };
}
