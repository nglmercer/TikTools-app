import type { ControlClient } from '@/shared/api/control-client.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { parseRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';

export type LibraryScope = 'public' | 'mine';

/**
 * Imports one template revision through the local library journal, with the Cloud
 * origin, account and revision as provenance. A public document is tried first; an
 * owner's private document is read from `mine` when the public read finds nothing.
 */
export interface LibraryDocumentRecord {
  readonly meta: { readonly sha256: string };
  readonly document: unknown;
}

/** Reads one published or owned revision. A public document is tried first, then the owner's private copy. */
export async function readLibraryDocument(client: ControlClient, id: string, revision: number): Promise<LibraryDocumentRecord> {
  try {
    return await client.call<LibraryDocumentRecord>('cloudLibrary.read', { scope: 'public', id, revision: String(revision) });
  } catch {
    return client.call<LibraryDocumentRecord>('cloudLibrary.read', { scope: 'mine', id, revision: String(revision) });
  }
}

/**
 * Imports one template revision through the local library journal, with the Cloud
 * origin, account and revision as provenance.
 */
export async function importLibraryTemplate(
  client: ControlClient,
  origin: string,
  accountId: string,
  id: string,
  revision: number,
): Promise<void> {
  const document = await readLibraryDocument(client, id, revision);
  const parsed = parseRuleTemplate(document.document);
  if (!parsed.ok) throw new Error(parsed.errors[0] ?? 'the template could not be read');
  await libraryJournal(client).request(
    { operation: 'import-templates', templates: [parsed.template] },
    null,
    { origin, accountId, documentId: id, revision: String(revision), sha256: document.meta.sha256 },
  );
}
