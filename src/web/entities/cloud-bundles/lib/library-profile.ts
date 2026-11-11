import { instantiateProfile, parseRuleProfileImport } from '@/entities/rule-template/model/rule-profiles-state.ts';
import type { LibraryMutation } from '@/entities/rule-template/model/library-contracts.ts';

/**
 * Turns one Cloud profile document into the library mutation the profile import
 * produces. The profile's default parameters are used; the user reviews the result
 * before it is sent, and imported automations stay disabled.
 */
export function profileMutation(document: unknown): LibraryMutation {
  const parsed = parseRuleProfileImport(JSON.stringify(document));
  if (!parsed.profile) throw new Error(parsed.errors[0] ?? 'the profile could not be read');
  return {
    operation: 'import-profile',
    profile: { id: parsed.profile.id, name: parsed.profile.name, description: parsed.profile.description },
    entries: instantiateProfile(parsed.profile),
  };
}
