/** Smoke launch modes do not change the updater's native installation detector. */
export type AppImageSmokeMode = 'mounted' | 'extracted';

export function parseAppImageSmokeArgs(args: readonly string[], platform: string) {
  const [imageArg, expectedVersion, flag, ...extra] = args;
  if (platform !== 'linux' || !imageArg || !expectedVersion || extra.length ||
      (flag !== undefined && flag !== '--extracted-mode')) {
    throw new Error('usage on Linux: bun scripts/smoke-appimage.ts <AppImage> <expected-version> [--extracted-mode]');
  }
  const mode: AppImageSmokeMode = flag === '--extracted-mode' ? 'extracted' : 'mounted';
  return { imageArg, expectedVersion, mode };
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
