export type PluginTargetOs = 'windows' | 'linux';
export type PluginTargetArch = 'x64' | 'arm64';

export type PluginBuildTarget = {
  rustTarget: string;
  pluginTarget: string;
  napiTarget: string;
  os: PluginTargetOs;
  arch: PluginTargetArch;
  executableExtension: string;
};

const PLUGIN_BUILD_TARGETS: Record<string, PluginBuildTarget> = {
  'x86_64-pc-windows-msvc': {
    rustTarget: 'x86_64-pc-windows-msvc',
    pluginTarget: 'win32-x64-msvc',
    napiTarget: 'win32-x64-msvc',
    os: 'windows',
    arch: 'x64',
    executableExtension: '.exe',
  },
  'aarch64-pc-windows-msvc': {
    rustTarget: 'aarch64-pc-windows-msvc',
    pluginTarget: 'win32-arm64-msvc',
    napiTarget: 'win32-arm64-msvc',
    os: 'windows',
    arch: 'arm64',
    executableExtension: '.exe',
  },
  'x86_64-unknown-linux-gnu': {
    rustTarget: 'x86_64-unknown-linux-gnu',
    pluginTarget: 'linux-x64-gnu',
    napiTarget: 'linux-x64-gnu',
    os: 'linux',
    arch: 'x64',
    executableExtension: '',
  },
  'aarch64-unknown-linux-gnu': {
    rustTarget: 'aarch64-unknown-linux-gnu',
    pluginTarget: 'linux-arm64-gnu',
    napiTarget: 'linux-arm64-gnu',
    os: 'linux',
    arch: 'arm64',
    executableExtension: '',
  },
};

export function supportedRustTargets(): string[] {
  return Object.keys(PLUGIN_BUILD_TARGETS);
}

export function supportedPluginTargets(): string[] {
  return Object.values(PLUGIN_BUILD_TARGETS).map((target) => target.pluginTarget);
}

export function resolveRustTarget(rustTarget: string): PluginBuildTarget {
  const normalized = rustTarget.trim();
  const target = PLUGIN_BUILD_TARGETS[normalized];
  if (!target) {
    throw new Error(
      `Unsupported Rust target "${rustTarget}". Supported targets: ${supportedRustTargets().join(', ')}`,
    );
  }
  return target;
}

export function pluginTargetFromRustTarget(rustTarget: string): string {
  return resolveRustTarget(rustTarget).pluginTarget;
}

export function napiTargetFromRustTarget(rustTarget: string): string {
  return resolveRustTarget(rustTarget).napiTarget;
}

export function detectHostTarget(
  platform: NodeJS.Platform = process.platform,
  arch: NodeJS.Architecture = process.arch,
): PluginBuildTarget {
  const key = hostRustTriple(platform, arch);
  if (!key) {
    throw new Error(
      `Unsupported host combination: platform "${platform}" arch "${arch}". ` +
        `Supported: win32/linux with x64/arm64. Pass --target <rust-target-triple> explicitly.`,
    );
  }
  return resolveRustTarget(key);
}

function hostRustTriple(platform: NodeJS.Platform, arch: NodeJS.Architecture): string | null {
  if (platform === 'win32' && arch === 'x64') return 'x86_64-pc-windows-msvc';
  if (platform === 'win32' && arch === 'arm64') return 'aarch64-pc-windows-msvc';
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  if (platform === 'linux' && arch === 'arm64') return 'aarch64-unknown-linux-gnu';
  return null;
}

/** File name segment (versions). Plugin ids use `plugin-id.ts`, not this pattern. */
const SAFE_FILENAME = /^[a-z0-9][a-z0-9._-]{0,63}$/i;

export function validatePluginVersion(version: unknown, file: string): string {
  if (
    typeof version !== 'string' ||
    version.trim().length === 0 ||
    version.length > 128 ||
    /[\s\\/]/.test(version)
  ) {
    throw new Error(`Plugin packaging failed: ${file} has no valid version for filenames`);
  }
  if (!SAFE_FILENAME.test(version)) {
    throw new Error(
      `Plugin packaging failed: ${file} has an unsafe version for filenames: ${version}`,
    );
  }
  return version;
}

export function artifactFileName(id: string, version: string, pluginTarget: string | null): string {
  if (pluginTarget) {
    return `${id}-${version}-${pluginTarget}.plugin`;
  }
  return `${id}-${version}.plugin`;
}
