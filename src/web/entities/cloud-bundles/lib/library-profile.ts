import type { ControlClient } from '@/shared/api/control-client.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { instantiateProfile, parseRuleProfileImport } from '@/entities/rule-template/model/rule-profiles-state.ts';
import { readLibraryDocument } from './library-import.ts';

/**
 * Imports one profile revision through the local library journal, as the rule-profile
 * import does. The profile's default parameters are used; the user reviews the result
 * in the profile list after import, and imported automations stay disabled.
 */
export async function importLibraryProfile(
  client: ControlClient,
  origin: string,
  accountId: string,
  id: string,
  revision: number,
): Promise<void> {
  const document = await readLibraryDocument(client, id, revision);
  const parsed = parseRuleProfileImport(JSON.stringify(document.document));
  if (!parsed.profile) throw new Error(parsed.errors[0] ?? 'the profile could not be read');
  const entries = instantiateProfile(parsed.profile);
  await libraryJournal(client).request(
    {
      operation: 'import-profile',
      profile: { id: parsed.profile.id, name: parsed.profile.name, description: parsed.profile.description },
      entries,
    },
    null,
    { origin, accountId, documentId: id, revision: String(revision), sha256: document.meta.sha256 },
  );
}
