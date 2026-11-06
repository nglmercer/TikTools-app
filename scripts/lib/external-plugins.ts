import { readdir, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';

/**
 * Optional external plugin checkout (`nglmercer/plugins-example-tiktools`).
 *
 * TikTools never requires it: builds, tests and startup work without it.
 * When present, development staging and `task.ts plugins` pick up its
 * packages. Resolution order:
 *
 * 1. `TIKTOOLS_PLUGINS_REPO` (an explicit path; empty or `off` disables).
 * 2. A sibling checkout `../plugins-example-tiktools`.
 */
export const PLUGINS_REPO_ENV = 'TIKTOOLS_PLUGINS_REPO';
export const PLUGINS_REPO_NAME = 'plugins-example-tiktools';

async function isDirectory(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isDirectory();
  } catch {
    return false;
  }
}

export async function resolvePluginsRepository(
  repositoryRoot: string,
  environment: Record<string, string | undefined> = process.env,
): Promise<string | null> {
  const explicit = environment[PLUGINS_REPO_ENV];
  if (explicit !== undefined) {
    const value = explicit.trim();
    if (value === '' || value.toLowerCase() === 'off') return null;
    const path = resolve(repositoryRoot, value);
    if (await isDirectory(join(path, 'plugins'))) return path;
    console.warn(`${PLUGINS_REPO_ENV}=${value} has no plugins/ directory; skipping external plugins.`);
    return null;
  }
  const sibling = resolve(repositoryRoot, '..', PLUGINS_REPO_NAME);
  return (await isDirectory(join(sibling, 'plugins'))) ? sibling : null;
}

/** Package directories (`<repo>/plugins/<name>/plugin.json`), sorted. */
export async function externalPluginDirectories(pluginsRepository: string): Promise<string[]> {
  const root = join(pluginsRepository, 'plugins');
  const directories: string[] = [];
  for (const entry of await readdir(root, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const directory = join(root, entry.name);
    try {
      if ((await stat(join(directory, 'plugin.json'))).isFile()) directories.push(directory);
    } catch {
      // Not a package directory.
    }
  }
  return directories.sort();
}
