import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dir, '..');
const SEMVER_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

function fail(message: string): never {
  console.error(`Version bump failed: ${message}`);
  process.exit(1);
}

function escapeRegExp(value: string): string {
  return value.replaceAll(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Strip an optional leading `v` and validate the result as semver. */
export function normalizeVersion(input: string): string {
  const version = input.replace(/^v/, '');
  if (!SEMVER_PATTERN.test(version)) {
    throw new Error(`expected a semver version such as 0.3.1, got ${JSON.stringify(input)}`);
  }
  return version;
}

export function assertVersionUpgrade(current: string, next: string): void {
  normalizeVersion(current);
  normalizeVersion(next);
  if (Bun.semver.order(next, current) <= 0) {
    throw new Error(`new version ${next} must have higher SemVer precedence than ${current}`);
  }
}

function extractSection(toml: string, header: string): { section: string; index: number; length: number } {
  const pattern = new RegExp(`\\[${escapeRegExp(header)}\\]([\\s\\S]*?)(?=\\n\\s*\\[[^\\]]+\\]|\\s*$)`);
  const match = toml.match(pattern);
  if (!match || match[1] === undefined || match.index === undefined) {
    throw new Error(`Cargo.toml has no [${header}] section`);
  }
  return { section: match[1], index: match.index, length: match[0].length };
}

/** Set `[workspace.package].version`, leaving dependency versions untouched. */
export function setWorkspaceVersion(cargoToml: string, version: string): string {
  const { section, index, length } = extractSection(cargoToml, 'workspace.package');
  const updated = section.replace(/^(\s*version\s*=\s*")[^"]+("\s*)$/m, `$1${version}$2`);
  if (updated === section) throw new Error('Cargo.toml has no [workspace.package].version');
  return `${cargoToml.slice(0, index)}[workspace.package]${updated}${cargoToml.slice(index + length)}`;
}

/** Read the current `[workspace.package].version`. */
export function readWorkspaceVersion(cargoToml: string): string {
  const { section } = extractSection(cargoToml, 'workspace.package');
  const version = section.match(/^\s*version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) throw new Error('Cargo.toml has no [workspace.package].version');
  return version;
}

/** Set the top-level `version` key, preserving key order and 2-space indent. */
export function setPackageJsonVersion(packageJsonText: string, version: string): string {
  const data = JSON.parse(packageJsonText) as Record<string, unknown>;
  data.version = version;
  return `${JSON.stringify(data, null, 2)}\n`;
}

/** Update a local workspace crate dependency without changing third-party pins. */
export function setPathDependencyVersions(text: string, names: string[], oldVersion: string, newVersion: string): string {
  return text.replace(/^(\s*([\w-]+)\s*=\s*\{[^\n]*\bpath\s*=\s*"[^"]+"[^\n]*\})$/gm, (line, _declaration, name: string) => {
    if (!names.includes(name)) return line;
    return line.replace(new RegExp(`(\\bversion\\s*=\\s*")${escapeRegExp(oldVersion)}(?=")`), `$1${newVersion}`);
  });
}

/** Workspace member directories from the root `[workspace] members` list. */
export function readWorkspaceMembers(cargoToml: string): string[] {
  const members = cargoToml.match(/\[workspace\][\s\S]*?members\s*=\s*\[([\s\S]*?)\]/)?.[1];
  if (members === undefined) throw new Error('Cargo.toml has no [workspace] members list');
  return [...members.matchAll(/"([^"]+)"/g)].map((match) => match[1] as string);
}

/** Read the `[package] name` of a member crate manifest. */
export function readPackageName(memberCargoToml: string): string {
  const { section } = extractSection(memberCargoToml, 'package');
  const name = section.match(/^\s*name\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!name) throw new Error('member Cargo.toml has no [package].name');
  return name;
}

/** Whether the member crate inherits `[workspace.package].version`. */
export function usesWorkspaceVersion(memberCargoToml: string): boolean {
  const { section } = extractSection(memberCargoToml, 'package');
  return /^\s*version\.workspace\s*=\s*true\s*$/m.test(section);
}

export interface CargoLockUpdate {
  text: string;
  updated: string[];
  missing: string[];
}

/**
 * Bump `version` for the given packages in Cargo.lock, but only where the
 * current version is `oldVersion`. Third-party crates are never touched.
 */
export function setCargoLockVersions(
  lockText: string,
  packageNames: string[],
  oldVersion: string,
  newVersion: string,
): CargoLockUpdate {
  let text = lockText;
  const updated: string[] = [];
  const missing: string[] = [];
  for (const name of packageNames) {
    const pattern = new RegExp(
      `(?<=name = "${escapeRegExp(name)}"\\nversion = ")${escapeRegExp(oldVersion)}(?=")`,
      'g',
    );
    const next = text.replace(pattern, newVersion);
    if (next === text) {
      missing.push(name);
    } else {
      updated.push(name);
      text = next;
    }
  }
  return { text, updated, missing };
}

/**
 * Standalone crates with their own lockfiles (test fixtures excluded from
 * the workspace). Their locks pin workspace crates by version,
 * so a bump must refresh them too or `--locked` builds fail.
 */
const STANDALONE_LOCK_PATTERNS = ['crates/*/tests/fixtures/*/Cargo.lock'];

export interface StandaloneLockResult {
  /** Repo-relative lockfile path with forward slashes. */
  path: string;
  updated: string[];
}

/**
 * Refresh workspace-crate pins in every standalone lockfile. Locks without
 * a matching entry (e.g. fixtures with no path dependencies) are left alone.
 */
export async function updateStandaloneLockfiles(
  root: string,
  packageNames: string[],
  oldVersion: string,
  newVersion: string,
): Promise<StandaloneLockResult[]> {
  const results: StandaloneLockResult[] = [];
  for (const pattern of STANDALONE_LOCK_PATTERNS) {
    const glob = new Bun.Glob(pattern);
    for await (const found of glob.scan({ cwd: root, onlyFiles: true })) {
      const relative = found.replaceAll('\\', '/');
      const absolute = resolve(root, relative);
      const text = await readFile(absolute, 'utf8');
      const update = setCargoLockVersions(text, packageNames, oldVersion, newVersion);
      if (update.text !== text) {
        await writeFile(absolute, update.text);
        results.push({ path: relative, updated: update.updated });
      }
    }
  }
  return results.sort((a, b) => (a.path < b.path ? -1 : 1));
}

function printHelp(): void {
  console.log(`Update the application version in Cargo.toml, Cargo.lock, package.json,
and standalone fixture/example lockfiles.

Usage:
  bun scripts/task.ts version bump <version>

The version accepts an optional leading "v": 0.3.1 and v0.3.1 are the same.
After bumping, commit the result and push the matching tag to cut a release.`);
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    printHelp();
    return;
  }
  const positional = args.filter((arg) => !arg.startsWith('-'));
  if (positional.length !== 1 || positional.length !== args.length) {
    fail('expected exactly one version argument; see --help');
  }
  const version = normalizeVersion(positional[0] as string);

  const cargoTomlPath = resolve(repositoryRoot, 'Cargo.toml');
  const cargoLockPath = resolve(repositoryRoot, 'Cargo.lock');
  const packageJsonPath = resolve(repositoryRoot, 'package.json');
  const packagerPath = resolve(repositoryRoot, 'Packager.toml');
  const [cargoToml, cargoLock, packageJsonText, packagerText] = await Promise.all([
    readFile(cargoTomlPath, 'utf8'),
    readFile(cargoLockPath, 'utf8'),
    readFile(packageJsonPath, 'utf8'),
    readFile(packagerPath, 'utf8'),
  ]);

  let current: string;
  let members: string[];
  try {
    current = readWorkspaceVersion(cargoToml);
    members = readWorkspaceMembers(cargoToml);
  } catch (error) {
    fail((error as Error).message);
  }
  if (current === version) {
    console.log(`Already at version ${version}; nothing to do.`);
    return;
  }
  try {
    assertVersionUpgrade(current, version);
  } catch (error) {
    fail((error as Error).message);
  }

  const packageNames: string[] = [];
  for (const member of members) {
    const memberToml = await readFile(resolve(repositoryRoot, member, 'Cargo.toml'), 'utf8').catch(() => {
      fail(`cannot read workspace member manifest at ${member}/Cargo.toml`);
    });
    try {
      // Members with an independent version (no version.workspace) keep it.
      if (usesWorkspaceVersion(memberToml)) packageNames.push(readPackageName(memberToml));
    } catch (error) {
      fail(`${member}/Cargo.toml: ${(error as Error).message}`);
    }
  }

  const lockUpdate = setCargoLockVersions(cargoLock, packageNames, current, version);
  const nextPackager = packagerText.replace(/^(version\s*=\s*")[^"]+("\s*)$/m, `$1${version}$2`);
  if (nextPackager === packagerText) fail('Packager.toml has no version or already has the requested version');
  let nextCargoToml: string;
  try {
    nextCargoToml = setWorkspaceVersion(cargoToml, version);
  } catch (error) {
    fail((error as Error).message);
  }
  await Promise.all([
    writeFile(cargoTomlPath, nextCargoToml),
    writeFile(cargoLockPath, lockUpdate.text),
    writeFile(packageJsonPath, setPackageJsonVersion(packageJsonText, version)),
    writeFile(packagerPath, nextPackager),
  ]);
  for (const member of members) {
    const path = resolve(repositoryRoot, member, 'Cargo.toml');
    const original = await readFile(path, 'utf8');
    const updated = setPathDependencyVersions(original, packageNames, current, version);
    if (updated !== original) await writeFile(path, updated);
  }
  const standalone = await updateStandaloneLockfiles(repositoryRoot, packageNames, current, version);

  console.log(`Bumped version: ${current} -> ${version}`);
  console.log('Updated Cargo.toml, Cargo.lock, and package.json.');
  for (const entry of standalone) {
    console.log(`Updated ${entry.path} (${entry.updated.join(', ')}).`);
  }
  if (lockUpdate.missing.length > 0) {
    console.warn(
      `Warning: no Cargo.lock entry at ${current} for: ${lockUpdate.missing.join(', ')}. ` +
        'Run a cargo command to refresh the lockfile if needed.',
    );
  }
  console.log(`Next steps to cut the release:\n  git add Cargo.toml Cargo.lock package.json\n  git commit -m "chore: release v${version}"\n  git tag v${version}\n  git push origin HEAD --tags`);
}

if (import.meta.main) {
  await main();
}
