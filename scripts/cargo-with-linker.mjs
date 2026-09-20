import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const WINDOWS_LINKER_ENVIRONMENTS = [
  'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER',
  'CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER',
];
const LINUX_LINKER_ENVIRONMENT_PATTERN = /^CARGO_TARGET_.*_LINUX_.*_LINKER$/;
const RUSTFLAGS_SEPARATOR = '\u001f';
const MOLD_RUSTFLAG = '-C link-arg=-fuse-ld=mold';
const ENCODED_MOLD_RUSTFLAGS = `-C${RUSTFLAGS_SEPARATOR}link-arg=-fuse-ld=mold`;

function commandExists(command, platform = process.platform) {
  const lookup = platform === 'win32'
    ? ['where.exe', [command]]
    : ['sh', ['-c', 'command -v "$1" >/dev/null 2>&1', 'sh', command]];

  return spawnSync(lookup[0], lookup[1], {
    stdio: 'ignore',
    windowsHide: true,
  }).status === 0;
}

function configuredLinuxLinkers(environment) {
  return Object.keys(environment).filter((name) => LINUX_LINKER_ENVIRONMENT_PATTERN.test(name));
}

function hasExplicitRustLinker(environment) {
  const rustFlags = [
    environment.RUSTFLAGS ?? '',
    environment.CARGO_ENCODED_RUSTFLAGS?.replaceAll(RUSTFLAGS_SEPARATOR, ' ') ?? '',
  ].join(' ');

  return rustFlags.includes('-fuse-ld=') || rustFlags.includes('linker=');
}

function addMoldRustFlag(environment) {
  if (Object.prototype.hasOwnProperty.call(environment, 'CARGO_ENCODED_RUSTFLAGS')) {
    const currentFlags = environment.CARGO_ENCODED_RUSTFLAGS ?? '';
    if (!currentFlags.includes('link-arg=-fuse-ld=mold')) {
      environment.CARGO_ENCODED_RUSTFLAGS = currentFlags
        ? `${currentFlags}${RUSTFLAGS_SEPARATOR}${ENCODED_MOLD_RUSTFLAGS}`
        : ENCODED_MOLD_RUSTFLAGS;
    }
    return;
  }

  const currentFlags = environment.RUSTFLAGS?.trim() ?? '';
  if (!currentFlags.includes('-fuse-ld=mold')) {
    environment.RUSTFLAGS = currentFlags ? `${currentFlags} ${MOLD_RUSTFLAG}` : MOLD_RUSTFLAG;
  }
}

export function resolveCargoEnvironment({
  environment = process.env,
  platform = process.platform,
  hasCommand = commandExists,
} = {}) {
  const cargoEnvironment = { ...environment };
  if (platform === 'linux') {
    const configuredTargets = configuredLinuxLinkers(environment);
    if (configuredTargets.length > 0 || hasExplicitRustLinker(environment)) {
      return { environment: cargoEnvironment, linker: 'unchanged', configuredTargets };
    }

    if (!hasCommand('mold', platform)) {
      return { environment: cargoEnvironment, linker: 'default', configuredTargets };
    }

    addMoldRustFlag(cargoEnvironment);
    return { environment: cargoEnvironment, linker: 'mold', configuredTargets };
  }

  if (platform !== 'win32') {
    return { environment: cargoEnvironment, linker: 'unchanged', configuredTargets: [] };
  }

  const unconfiguredTargets = WINDOWS_LINKER_ENVIRONMENTS.filter(
    (name) => !Object.prototype.hasOwnProperty.call(environment, name),
  );
  const configuredTargets = WINDOWS_LINKER_ENVIRONMENTS.filter(
    (name) => !unconfiguredTargets.includes(name),
  );

  if (unconfiguredTargets.length === 0 || !hasCommand('lld-link.exe')) {
    return { environment: cargoEnvironment, linker: 'default', configuredTargets };
  }

  for (const name of unconfiguredTargets) {
    cargoEnvironment[name] = 'lld-link';
  }
  return { environment: cargoEnvironment, linker: 'lld-link', configuredTargets };
}

function main() {
  const cargoArguments = process.argv.slice(2);
  if (cargoArguments.length === 0) {
    console.error('Usage: node scripts/cargo-with-linker.mjs <cargo arguments...>');
    process.exitCode = 2;
    return;
  }

  const resolved = resolveCargoEnvironment();
  if (process.platform === 'win32') {
    const preserved = resolved.configuredTargets.length > 0
      ? `; preserved ${resolved.configuredTargets.join(', ')}`
      : '';
    if (resolved.linker === 'lld-link') {
      console.error(`[cargo-with-linker] using lld-link${preserved}`);
    } else if (resolved.configuredTargets.length > 0) {
      console.error(`[cargo-with-linker] preserving user linker configuration${preserved}`);
    } else {
      console.error('[cargo-with-linker] lld-link unavailable; using default MSVC linker');
    }
  } else if (process.platform === 'linux') {
    if (resolved.linker === 'mold') {
      console.error('[cargo-with-linker] using mold');
    } else if (resolved.linker === 'default') {
      console.error('[cargo-with-linker] mold unavailable; using default Linux linker');
    }
  }

  const result = spawnSync('cargo', cargoArguments, {
    env: resolved.environment,
    stdio: 'inherit',
    windowsHide: false,
  });
  if (result.error) {
    console.error(`Failed to start Cargo: ${result.error.message}`);
    process.exitCode = 1;
    return;
  }
  process.exitCode = result.status ?? 1;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : '';
if (invokedPath === fileURLToPath(import.meta.url)) {
  main();
}
