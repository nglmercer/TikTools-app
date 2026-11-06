import { normalizeVersion } from '../bump-version.ts';

/** Decide which authenticated feed heads may advance. Missing heads are empty. */
export function selectUpdateChannels(candidate: string, stable: string | null, beta: string | null): Array<'stable' | 'beta'> {
  const next = normalizeVersion(candidate);
  if (next !== candidate) throw new Error('candidate must be an unprefixed SemVer');
  for (const current of [stable, beta]) {
    if (current !== null && normalizeVersion(current) !== current) throw new Error('feed head must be an unprefixed SemVer');
  }
  const advances = (current: string | null) => current === null || Bun.semver.order(next, current) > 0;
  const result: Array<'stable' | 'beta'> = [];
  // Build metadata can contain a hyphen; only the prerelease part matters.
  if (!next.split('+', 1)[0]!.includes('-') && advances(stable)) result.push('stable');
  if (advances(beta)) result.push('beta');
  return result;
}
