export type RecoveryValue = { state: 'absent' | 'deleted' | 'selected'; version: string | null; enabled: boolean | null };
export type RecoveryReview = { handle: string; key: string; base: RecoveryValue; original: RecoveryValue; local: RecoveryValue; cloud: RecoveryValue };
export type V2State = 'syncDisabled' | 'signInRequired' | 'backendNotSupported' | 'backendCompatible';
export type V2Preview = { selections: number; pending: number; conflicts: number; retainedV1Only: number; migrationAvailable: boolean };
export type V2Migration = { selections: number; pending: number; conflicts: number; retainedV1Only: number; completed: boolean; policyActivated: boolean };
export type V2Result = { pushed: number; pulled: number; workspacePending: number; availableRecords: number; conflicts: string[]; moreQueued: boolean };
export type V2ConflictTarget = { collection: string; key: string };
export type V2ConflictValue = { state: 'absent' | 'deleted' | 'present'; revision: number | null; value: unknown; version: string | null; enabled: boolean | null };
export type V2ConflictReview = { handle: string; collection: string; key: string; base: V2ConflictValue; local: V2ConflictValue; cloud: V2ConflictValue; hasNewer: boolean };

export function validRecoveryValue(value: unknown): value is RecoveryValue {
  if (value == null || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  if (candidate.state === 'absent' || candidate.state === 'deleted') {
    return candidate.version === null && candidate.enabled === null;
  }
  return candidate.state === 'selected' && typeof candidate.version === 'string'
    && candidate.version.length > 0 && typeof candidate.enabled === 'boolean';
}

export function validRecoveryReview(value: unknown, key: string): value is RecoveryReview {
  if (value == null || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.handle === 'string' && /^[0-9a-f]{64}$/.test(candidate.handle)
    && candidate.key === key && validRecoveryValue(candidate.base)
    && validRecoveryValue(candidate.original) && validRecoveryValue(candidate.local)
    && validRecoveryValue(candidate.cloud);
}

export function validV2Value(value: unknown): value is V2ConflictValue {
  if (value == null || typeof value !== 'object' || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  if (candidate.state !== 'absent' && candidate.state !== 'deleted' && candidate.state !== 'present') return false;
  if (candidate.revision !== null && !Number.isSafeInteger(candidate.revision)) return false;
  if (candidate.version !== null && typeof candidate.version !== 'string') return false;
  if (candidate.enabled !== null && typeof candidate.enabled !== 'boolean') return false;
  return true;
}

export function validV2Review(value: unknown, collection: string, key: string): value is V2ConflictReview {
  if (value == null || typeof value !== 'object' || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.handle === 'string' && /^[0-9a-f]{64}$/.test(candidate.handle)
    && candidate.collection === collection && candidate.key === key
    && typeof candidate.hasNewer === 'boolean' && validV2Value(candidate.base)
    && validV2Value(candidate.local) && validV2Value(candidate.cloud);
}
