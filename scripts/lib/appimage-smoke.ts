/** Smoke launch modes do not change the updater's native installation detector. */
export type AppImageSmokeMode = 'mounted' | 'extracted';

export function parseAppImageSmokeArgs(args: readonly string[], platform: string) {
  const [imageArg, expectedVersion, ...flags] = args;
  if (platform !== 'linux' || !imageArg || !expectedVersion || new Set(flags).size !== flags.length || flags.some(flag => !['--extracted-mode', '--require-cli'].includes(flag))) {
    throw new Error('usage on Linux: bun scripts/smoke-appimage.ts <AppImage> <expected-version> [--extracted-mode] [--require-cli]');
  }
  const mode: AppImageSmokeMode = flags.includes('--extracted-mode') ? 'extracted' : 'mounted';
  return { imageArg, expectedVersion, mode, requireCli: flags.includes('--require-cli') };
}

export function checkAppImageSmokeInstallation(output: string, mode: AppImageSmokeMode): void {
  let kind: unknown;
  try { kind = JSON.parse(output); }
  catch { throw new Error(`AppImage ${mode} installation probe returned invalid JSON: ${JSON.stringify(output)}`); }
  // The immutable native detector requires a matching FUSE mount and image.
  // An extracted launch intentionally fails closed for automatic installation.
  const allowed = mode === 'mounted' ? ['appImage'] : ['unknown', 'portable'];
  if (typeof kind !== 'string' || !allowed.includes(kind)) {
    throw new Error(`AppImage ${mode} installation detection failed: received ${JSON.stringify(kind)}; expected ${allowed.join(' or ')}`);
  }
}

export function checkAppImageCli(output: string, version: string): void {
  const value = JSON.parse(output) as { version?: string; features?: { cloudAuth?: boolean; remoteClient?: boolean; remoteControl?: boolean } };
  if (value.version !== version || !value.features?.cloudAuth || !value.features.remoteClient || !value.features.remoteControl) throw new Error('AppImage headless CLI is missing or has incompatible features');
}
