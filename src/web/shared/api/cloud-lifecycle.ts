import { ControlCallError } from './control-client.ts';

/**
 * What a failed Cloud lifecycle call (list, rename, delete, fork, stage variables) means for the
 * person. The host sets a stable `code` and, for the cases that need an explanation, a
 * `data.reason`; this maps them to one small set of kinds the panels translate. Nothing here
 * reads a message, so the wording never depends on the server's text.
 */
export type LifecycleFailureKind =
  /** An overlay revision still names the image: remove it from the overlay first. */
  | 'assetReferenced'
  /** A package still pins the overlay: delete the package first. */
  | 'overlayPinned'
  | 'alreadyInLibrary'
  | 'notPublished'
  /** The account's storage limit is reached. */
  | 'quota'
  | 'libraryQuota'
  | 'rateLimited'
  /** The Cloud instance does not offer this feature (the host checked its capability document). */
  | 'unsupported'
  | 'notFound'
  | 'signIn'
  | 'other';

function reasonOf(failure: ControlCallError): string {
  const data = failure.data;
  if (typeof data !== 'object' || data === null) return '';
  const reason = (data as { reason?: unknown }).reason;
  return typeof reason === 'string' ? reason : '';
}

export function lifecycleFailure(failure: unknown): LifecycleFailureKind {
  if (!(failure instanceof ControlCallError)) return 'other';
  const reason = reasonOf(failure);
  switch (failure.code) {
    case 'conflict':
      if (reason === 'assetReferenced' || reason === 'overlayPinned' || reason === 'alreadyInLibrary') return reason;
      return 'other';
    case 'quota_exceeded':
      return reason === 'libraryQuota' ? 'libraryQuota' : 'quota';
    case 'capability_unavailable':
      return 'unsupported';
    case 'not_found':
      return reason === 'notPublished' ? 'notPublished' : 'notFound';
    case 'unavailable':
      return reason === 'rateLimited' ? 'rateLimited' : 'other';
    case 'forbidden':
    case 'signed_out':
    case 'unauthorized':
      return 'signIn';
    default:
      return 'other';
  }
}

/** Locale key (under `cloudLifecycle.failure`) of the plain message for a kind, or null for `other`. */
export function lifecycleMessageKey(kind: LifecycleFailureKind): string | null {
  return kind === 'other' ? null : `cloudLifecycle.failure.${kind}`;
}
