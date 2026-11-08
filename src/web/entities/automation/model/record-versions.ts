import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';

export type AutomationKind = 'event' | 'action';
export type AutomationRecord = LiveAction | LiveEvent;
// null is allowed only for an explicitly identified non-persistent owner host.
export type AutomationRevision = string | null;
export interface AutomationHost { hostId: string; hostInstanceId: string }
export interface AutomationWriteBase { revision: AutomationRevision; create: boolean; host?: AutomationHost | null }
export interface AutomationRecordVersion { record: AutomationRecord | null; revision: AutomationRevision; host?: AutomationHost | null }

export const recordKey = (kind: AutomationKind, id: string): string => `${kind}:${id}`;
export const validRevision = (value: unknown): value is string =>
  typeof value === 'string' && /^(0|[1-9][0-9]{0,18})$/.test(value) && BigInt(value) <= 9_223_372_036_854_775_807n;

export function automationVersions(meta: unknown): Map<string, string> {
  const versions = new Map<string, string>();
  if (meta === undefined) return versions;
  if (!meta || typeof meta !== 'object' || !('resourceVersions' in meta) || !Array.isArray(meta.resourceVersions)) {
    throw new ControlCallError('protocol', 'Missing automation revision metadata.');
  }
  for (const v of meta.resourceVersions as unknown[]) {
    if (!v || typeof v !== 'object' || !('kind' in v) || v.kind !== 'automation' ||
        !('scope' in v) || (v.scope !== 'action' && v.scope !== 'event') ||
        !('id' in v) || typeof v.id !== 'string' || !('revision' in v) || !validRevision(v.revision)) {
      throw new ControlCallError('protocol', 'Invalid automation revision metadata.');
    }
    const key = recordKey(v.scope as AutomationKind, v.id);
    if (versions.has(key)) throw new ControlCallError('protocol', 'Ambiguous automation revision metadata.');
    versions.set(key, v.revision);
  }
  return versions;
}

export function recordRevision(meta: unknown, kind: AutomationKind, id: string, memory: boolean): AutomationRevision {
  const revision = automationVersions(meta).get(recordKey(kind, id));
  if (revision !== undefined) return revision;
  if (memory) return null;
  throw new ControlCallError('protocol', 'The host did not confirm the automation revision. Read and review the current record.');
}

export function assertRecord(record: AutomationRecord, kind: AutomationKind, id: string): void {
  if (!record || typeof record !== 'object' || record.id !== id ||
      (kind === 'event' ? !('trigger' in record) : !('typeId' in record))) {
    throw new ControlCallError('protocol', 'The host returned a different automation record.');
  }
}
