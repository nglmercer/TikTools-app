import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import type { PluginBuildTarget } from './plugin-targets.ts';

/**
 * Shared, bounded steps of the napi-vm pipeline used by development staging
 * (`scripts/dev-plugins.ts`) and by packaging (`scripts/build-plugin.ts`):
 * compile the TypeScript guest, then make the declared native addons
 * available to the package directory.
 *
 * The steps take explicit target, profile, directory and executor
 * parameters. Policy stays with the caller: development skips a package
 * whose native sources are unavailable, packaging fails; development writes
 * under `.dev-plugins`, packaging produces the `.plugin` archive.
 */

export type NapiAddon = { packageName: string; root: string };

export type CargoProfile = 'debug' | 'release';

/** Runs `command` with `args`, rejecting (or throwing) when it fails. */
export type Exec = (command: string, args: string[]) => void | Promise<void>;

export type NativeSourcePlan =
  /** Every declared package has a sibling checkout and the build targets this host. */
  | { kind: 'sibling' }
  /** Host binaries are fetched from the providers pinned by `native-libs.lock.json`. */
  | { kind: 'fetch' }
  | { kind: 'unavailable'; reason: 'cross-target' | 'missing-sibling'; missing: string[] };

const NPM_PACKAGE_NAME = /^(@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/;

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

function isSafeRelativeRoot(value: string): boolean {
  return (
    value.length > 0 &&
    !value.includes('\0') &&
    !value.includes('\\') &&
    !value.startsWith('/') &&
    !value.split('/').includes('..')
  );
}

/** The `nativeAddons` declarations of a manifest, validated. Empty when none are declared. */
export function readNativeAddons(manifest: Record<string, unknown>, manifestPath: string): NapiAddon[] {
  const declared = manifest['nativeAddons'];
  if (declared === undefined) return [];
  if (!Array.isArray(declared)) throw new Error(`${manifestPath} has a malformed nativeAddons list`);
  return declared.map((entry: { package?: unknown; root?: unknown } | null) => {
    const packageName = entry?.package;
    const root = entry?.root;
    if (
      typeof packageName !== 'string' ||
      !NPM_PACKAGE_NAME.test(packageName) ||
      typeof root !== 'string' ||
      !isSafeRelativeRoot(root)
    ) {
      throw new Error(`${manifestPath} has a malformed nativeAddons entry`);
    }
    return { packageName, root };
  });
}

/** Where a sibling checkout of a native package is expected: beside the repository. */
export function siblingSourceDirectory(repositoryRoot: string, packageName: string): string {
  return join(repositoryRoot, '..', packageName);
}

/** Argv (after `node`) for compiling the guest with the repository `tsc`. Types resolve from TikTools when the plugin checkout has no `node_modules`. */
export function typescriptCompilerArgs(repositoryRoot: string, tsconfigPath: string): string[] {
  return [
    join(repositoryRoot, 'node_modules', 'typescript', 'bin', 'tsc'),
    '-p',
    tsconfigPath,
    '--typeRoots',
    join(repositoryRoot, 'node_modules', '@types'),
  ];
}

/** Compiles the napi-vm guest of `packageDirectory`; fails when it has no `tsconfig.json`. */
export async function compileNapiVmGuest(options: {
  repositoryRoot: string;
  packageDirectory: string;
  manifestPath: string;
  exec: Exec;
}): Promise<void> {
  const tsconfigPath = join(options.packageDirectory, 'tsconfig.json');
  if (!(await exists(tsconfigPath))) {
    throw new Error(`${options.manifestPath} is napi-vm but has no tsconfig.json`);
  }
  await options.exec('node', typescriptCompilerArgs(options.repositoryRoot, tsconfigPath));
}

/** Cargo arguments that build the native staging tool, which always runs on the host. */
export function stageNativeBuildArgs(profile: CargoProfile): string[] {
  return [
    'build',
    ...(profile === 'release' ? ['--release'] : []),
    '-p',
    'tiktools-plugin-devkit',
    '--bin',
    'tiktools-plugin-stage-native',
  ];
}

/** Path of the staging tool: host platform suffix, never the build target's. */
export function stageNativeExecutable(
  repositoryRoot: string,
  profile: CargoProfile,
  host: PluginBuildTarget,
): string {
  return join(repositoryRoot, 'target', profile, `tiktools-plugin-stage-native${host.executableExtension}`);
}

export async function pinnedLockfileAvailable(manifest: Record<string, unknown>, lockfilePath: string): Promise<boolean> {
  const nativeLibs = manifest['nativeLibs'];
  return Array.isArray(nativeLibs) && nativeLibs.length > 0 && (await exists(lockfilePath));
}

/**
 * Choose where native binaries come from. A local sibling checkout wins
 * for host builds (fast, offline); a cross-target build can never use it
 * because the sibling holds host binaries. Otherwise a pinned lockfile
 * fetches the requested target from its provider. Pure.
 */
export function planNativeSources(input: {
  missingSiblings: string[];
  canFetch: boolean;
  crossTarget: boolean;
}): NativeSourcePlan {
  if (!input.crossTarget && input.missingSiblings.length === 0) return { kind: 'sibling' };
  if (input.canFetch) return { kind: 'fetch' };
  return input.crossTarget
    ? { kind: 'unavailable', reason: 'cross-target', missing: [] }
    : { kind: 'unavailable', reason: 'missing-sibling', missing: input.missingSiblings };
}

/** Staging-tool invocations (argv without the executable) for an available plan. Pure. */
export function nativeStageInvocations(input: {
  plan: Extract<NativeSourcePlan, { kind: 'sibling' | 'fetch' }>;
  repositoryRoot: string;
  packageDirectory: string;
  manifestPath: string;
  lockfilePath: string;
  addons: NapiAddon[];
  target: PluginBuildTarget;
}): string[][] {
  if (input.plan.kind === 'sibling') {
    return input.addons.map((addon) => [
      '--package',
      addon.packageName,
      '--source',
      siblingSourceDirectory(input.repositoryRoot, addon.packageName),
      '--plugin-dir',
      input.packageDirectory,
      '--root',
      addon.root,
      '--overwrite',
    ]);
  }
  return [
    [
      '--provider',
      'all',
      '--manifest',
      input.manifestPath,
      '--plugin-dir',
      input.packageDirectory,
      '--lockfile',
      input.lockfilePath,
      '--target',
      input.target.napiTarget,
      '--overwrite',
    ],
  ];
}

export type NativeAddonPreparation =
  | { status: 'none' }
  | { status: 'staged'; source: 'sibling' | 'fetch' }
  | { status: 'unavailable'; reason: 'cross-target' | 'missing-sibling'; missing: string[] };

/**
 * Prepares the declared native addons of a napi-vm package. Builds the
 * staging tool for `profile` on the host, then runs it for `target`. Returns
 * `unavailable` (without building anything) when no source can provide the
 * binaries, leaving the skip-or-fail decision to the caller.
 */
export async function prepareNativeAddons(options: {
  repositoryRoot: string;
  packageDirectory: string;
  manifestPath: string;
  manifest: Record<string, unknown>;
  host: PluginBuildTarget;
  target: PluginBuildTarget;
  profile: CargoProfile;
  /** Runs the repository cargo wrapper with these arguments. */
  cargo: (args: string[]) => void | Promise<void>;
  /** Runs the freshly built staging tool with these arguments. */
  exec: Exec;
}): Promise<NativeAddonPreparation> {
  const addons = readNativeAddons(options.manifest, options.manifestPath);
  if (addons.length === 0) return { status: 'none' };

  const missingSiblings: string[] = [];
  for (const addon of addons) {
    if (!(await exists(join(siblingSourceDirectory(options.repositoryRoot, addon.packageName), 'package.json')))) {
      missingSiblings.push(addon.packageName);
    }
  }
  const lockfilePath = join(options.packageDirectory, 'native-libs.lock.json');
  const plan = planNativeSources({
    missingSiblings,
    canFetch: await pinnedLockfileAvailable(options.manifest, lockfilePath),
    crossTarget: options.target.rustTarget !== options.host.rustTarget,
  });
  if (plan.kind === 'unavailable') return { status: 'unavailable', reason: plan.reason, missing: plan.missing };

  await options.cargo(stageNativeBuildArgs(options.profile));
  const stageBinary = stageNativeExecutable(options.repositoryRoot, options.profile, options.host);
  for (const args of nativeStageInvocations({
    plan,
    repositoryRoot: options.repositoryRoot,
    packageDirectory: options.packageDirectory,
    manifestPath: options.manifestPath,
    lockfilePath,
    addons,
    target: options.target,
  })) {
    await options.exec(stageBinary, args);
  }
  return { status: 'staged', source: plan.kind };
}
