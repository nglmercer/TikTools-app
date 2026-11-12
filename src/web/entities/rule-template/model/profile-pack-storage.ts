import { DEFAULT_PROFILE_ID, type ProfilePack } from '../lib/rule-profiles.ts';
import { isRecord, readText } from '../lib/profile-values.ts';

export const PROFILE_PACKS_KEY = 'behavior.profiles.packs';
export const ACTIVE_PROFILE_KEY = 'behavior.profiles.active';

function asPack(value: unknown): ProfilePack | null {
  if (!isRecord(value)) return null;
  const id = readText(value['id'], 64);
  const name = readText(value['name'], 80);
  const eventIds = value['eventIds'];
  const actionIds = value['actionIds'];
  if (!id || !name || !Array.isArray(eventIds) || !Array.isArray(actionIds)) return null;
  if (!eventIds.every((entry) => typeof entry === 'string') || !actionIds.every((entry) => typeof entry === 'string')) {
    return null;
  }
  return {
    id,
    name,
    description: typeof value['description'] === 'string' ? (value['description'] as string).slice(0, 500) : '',
    eventIds: [...eventIds] as string[],
    actionIds: [...actionIds] as string[],
  };
}

export function parseStoredProfilePacks(raw: string | undefined): { packs: ProfilePack[]; errors: string[] } {
  if (!raw || raw.trim() === '') return { packs: [], errors: [] };
  let value: unknown;
  try {
    value = JSON.parse(raw) as unknown;
  } catch {
    return { packs: [], errors: ['stored profiles are not valid JSON'] };
  }
  if (!Array.isArray(value)) return { packs: [], errors: ['stored profiles must be an array'] };
  const packs: ProfilePack[] = [];
  const errors: string[] = [];
  value.forEach((entry: unknown, index: number) => {
    const pack = asPack(entry);
    if (!pack) {
      errors.push(`stored profiles[${index + 1}] is malformed (dropped)`);
    } else if (pack.id === DEFAULT_PROFILE_ID) {
      errors.push(`stored profiles[${index + 1}] uses the reserved default id (dropped)`);
    } else {
      packs.push(pack);
    }
  });
  return { packs, errors };
}
