import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const WINDOWS_LINKER_ENVIRONMENTS = [
  'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER',
  'CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER',
];

function commandExists(command) {
  return spawnSync('where.exe', [command], {
    stdio: 'ignore',
    windowsHide: true,
  }).status === 0;
}

export function resolveCargoEnvironment({
  environment = process.env,
  platform = process.platform,
  hasCommand = commandExists,
} = {}) {
  const cargoEnvironment = { ...environment };
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
