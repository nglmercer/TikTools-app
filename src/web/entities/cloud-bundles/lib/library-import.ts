import { ControlCallError, type ControlClient } from '@/shared/api/control-client.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { sameLibraryHost, type LibraryCloudProvenance, type LibraryMutation } from '@/entities/rule-template/model/library-contracts.ts';
import { parseRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { profileMutation } from './library-profile.ts';

export type LibraryScope = 'public' | 'mine';
export type LibraryImportKind = 'template' | 'profile';

export interface LibraryDocumentRecord {
  readonly meta: { readonly id?: string; readonly revision?: string | number; readonly sha256: string };
  readonly document: unknown;
}

/** Failures the coordinator raises itself; each carries a stable `code` for the caller's own wording. */
export type LibraryImportFailureCode = 'signed_out' | 'stale_context' | 'invalid_document';
export class LibraryImportError extends ControlCallError {
  constructor(code: LibraryImportFailureCode, message: string) {
    super(code, message);
    this.name = 'LibraryImportError';
  }
}

/** What the coordinator needs to know about the Cloud account and the connection it runs against. */
export interface LibraryImportEnvironment {
  readonly client: ControlClient;
  /** The signed-in account, or `null` when there is none. */
  identity(): { readonly origin: string; readonly accountId: string } | null;
  /** Opaque marker of the current account/session/host context. */
  stamp(): number;
  /** Whether nothing that affects the import has changed since `stamp` was taken. */
  isCurrent(stamp: number): boolean;
}

export interface LibraryImportSource {
  readonly id: string;
  readonly revision: string | number;
  /** When the caller already knows where the document lives. Unknown scopes try the public copy first. */
  readonly scope?: LibraryScope;
  /** A document the caller already read and validated in the current context; it is not read again. */
  readonly record?: LibraryDocumentRecord;
}

const SHA256 = /^[0-9a-f]{64}$/;

/**
 * Reads one published or owned revision. With a known scope that scope is the only one
 * asked. Otherwise the public copy is tried first and the owner's private copy only
 * when the host says the public document does not exist (`not_found`); network,
 * permission or shape failures are never retried against another scope.
 */
export async function readLibraryDocument(
  client: ControlClient,
  id: string,
  revision: string | number,
  scope?: LibraryScope,
): Promise<LibraryDocumentRecord> {
  const read = (target: LibraryScope) => client.call<LibraryDocumentRecord>('cloudLibrary.read', { scope: target, id, revision: String(revision) });
  if (scope) return read(scope);
  try {
    return await read('public');
  } catch (failure) {
    if (failure instanceof ControlCallError && failure.code === 'not_found') return read('mine');
    throw failure;
  }
}

function templateMutation(document: unknown): LibraryMutation {
  const parsed = parseRuleTemplate(document);
  if (!parsed.ok) throw new Error(parsed.errors[0] ?? 'the template could not be read');
  return { operation: 'import-templates', templates: [parsed.template] };
}

function checkRecord(record: LibraryDocumentRecord, id: string, revision: string): void {
  const { meta } = record;
  if (!meta || typeof meta.sha256 !== 'string' || !SHA256.test(meta.sha256)) throw new LibraryImportError('invalid_document', 'the Cloud document has no valid digest');
  if (meta.id !== undefined && meta.id !== id) throw new LibraryImportError('invalid_document', 'the Cloud document is not the one requested');
  if (meta.revision !== undefined && String(meta.revision) !== revision) throw new LibraryImportError('invalid_document', 'the Cloud document is not the revision requested');
}

/**
 * The one import of a Cloud template or profile. It captures the account and the
 * connection, reads the document and a coherent library snapshot of the current host,
 * revalidates the context after every asynchronous step and only then asks the owner
 * journal for the review, with the real provenance. The returned promise settles when
 * the user confirms or discards that review; nothing is resent after a transport error.
 */
export async function importLibraryDocument(
  environment: LibraryImportEnvironment,
  kind: LibraryImportKind,
  source: LibraryImportSource,
): Promise<void> {
  const identity = environment.identity();
  if (!identity) throw new LibraryImportError('signed_out', `sign in to Cloud to import this ${kind}`);
  const stamp = environment.stamp();
  const stale = () => new LibraryImportError('stale_context', 'the Cloud account or the host changed while the document was being read; open it again');
  const revision = String(source.revision);
  const journal = libraryJournal(environment.client);
  // The host that answers before the document is read must be the host that answers after it.
  const before = source.record ? null : await journal.readSnapshot();
  const record = source.record ?? await readLibraryDocument(environment.client, source.id, revision, source.scope);
  checkRecord(record, source.id, revision);
  const mutation = kind === 'template' ? templateMutation(record.document) : profileMutation(record.document);
  const context = await journal.refreshSnapshot();
  const current = environment.identity();
  if (!environment.isCurrent(stamp) || !current || current.origin !== identity.origin || current.accountId !== identity.accountId
    || (before && !sameLibraryHost(before, context))) throw stale();
  const provenance: LibraryCloudProvenance = {
    origin: identity.origin, accountId: identity.accountId, documentId: source.id, revision, sha256: record.meta.sha256,
  };
  return journal.request(mutation, context, provenance);
}
