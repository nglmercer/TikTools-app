export type CliBuildInfo = { version: string; sourceRevision: string; buildMode: string; features: { cloudAuth: boolean; remoteClient: boolean; remoteControl: boolean } };

/** Shipping a same-version CLI from a different checkout is still a broken release. */
export function validateCliBuildInfo(value: unknown, version: string, revision: string): asserts value is CliBuildInfo {
  if (!value || typeof value !== 'object') throw new Error('CLI build metadata is missing');
  const build = value as Partial<CliBuildInfo>;
  if (build.version !== version || build.sourceRevision !== revision || build.buildMode !== 'release') throw new Error('CLI version, source revision or build mode differs from the release');
  if (!build.features?.cloudAuth || !build.features.remoteClient || !build.features.remoteControl) throw new Error('release CLI lacks Cloud/controller/headless host support');
}
