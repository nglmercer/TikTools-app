import { ControlCallError, type CommandReceipt, type ResourceVersion } from '@/shared/api/control-client.ts';
import { sourceNumbersSafe } from '@/shared/lib/json-source.ts';
import { assertRecord, validRevision, type AutomationKind, type AutomationRecord } from './record-versions.ts';

export type DefinitionOperation = 'create' | 'update' | 'delete' | 'enable' | 'disable';
export interface DefinitionIntent {
  commandId: string; operation: DefinitionOperation; kind: AutomationKind; id: string;
  expectedRevision: string; hostInstanceId: string; recordDigest: string | null; createdAt: number;
}
export interface DefinitionReceipt {
  revision: string; recordDigest: string | null; enabled: boolean | null; deleted: boolean | null; affectedResources: number;
}
export interface DefinitionRecord {
  intent: DefinitionIntent; outcome: 'succeeded' | 'rejected' | 'reviewed-unknown';
  receipt: DefinitionReceipt | null; errorCode: string | null; recordedAt: number;
}
export interface DefinitionState {
  hostId: string; hostInstanceId: string; pending: DefinitionIntent | null; recent: DefinitionRecord[];
}
export interface DefinitionAcknowledgement {
  value: AutomationRecord | { id: string; deleted: boolean };
  resourceVersions: ResourceVersion[]; command: CommandReceipt; hostInstanceId: string;
}
export interface DefinitionSubmission { state: DefinitionState; acknowledgement: DefinitionAcknowledgement | null }
export type DefinitionWrite = { kind: AutomationKind; id: string; expectedRevision: string } & (
  | { operation: 'create' | 'update'; record: AutomationRecord }
  | { operation: 'delete' | 'enable' | 'disable' }
);

export const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
export const exact = (v: unknown, keys: readonly string[]): v is Record<string, unknown> => object(v) && Object.keys(v).length === keys.length && keys.every((k) => Object.hasOwn(v, k));
const text = (v: unknown, bytes = 256): v is string => typeof v === 'string' && !!v && v.trim() === v && new TextEncoder().encode(v).length <= bytes && !/\p{Cc}/u.test(v);
const id = (v: unknown): v is string => text(v, 128) && !v.includes('*');
const digest = (v: unknown): v is string => typeof v === 'string' && /^[0-9a-f]{64}$/.test(v);
const timestamp = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const kinds = ['action', 'event'];
const operations = ['create', 'update', 'delete', 'enable', 'disable'];
const rejected = ['revision_conflict', 'not_found', 'invalid_params', 'forbidden', 'capability_unavailable', 'host_restarted', 'too_large'];
const invalid = (): never => { throw new ControlCallError('protocol', 'Invalid native Owner definition response.'); };
const saving = (operation: DefinitionOperation): boolean => operation === 'create' || operation === 'update';
const stableJson = (value: unknown): string => JSON.stringify(value, (_key, node: unknown) => object(node)
  ? Object.fromEntries(Object.keys(node).sort().map((key) => [key, node[key]])) : node);
const utf8Compare = (a: string, b: string): number => {
  const left = new TextEncoder().encode(a); const right = new TextEncoder().encode(b);
  for (let i = 0; i < Math.min(left.length, right.length); ++i) if (left[i] !== right[i]) return left[i]! - right[i]!;
  return left.length - right.length;
};

function parseIntent(v: unknown): DefinitionIntent {
  if (!exact(v, ['commandId', 'operation', 'kind', 'id', 'expectedRevision', 'hostInstanceId', 'recordDigest', 'createdAt']) ||
      typeof v.commandId !== 'string' || !/^owner-definition-[0-9a-f]{32}$/.test(v.commandId) ||
      typeof v.operation !== 'string' || !operations.includes(v.operation) || typeof v.kind !== 'string' || !kinds.includes(v.kind) ||
      !id(v.id) || !validRevision(v.expectedRevision) ||
      !text(v.hostInstanceId) || !timestamp(v.createdAt) ||
      (saving(v.operation as DefinitionOperation) ? !digest(v.recordDigest) : v.recordDigest !== null)) return invalid();
  return v as unknown as DefinitionIntent;
}
function parseReceipt(v: unknown, intent: DefinitionIntent): DefinitionReceipt {
  if (!exact(v, ['revision', 'recordDigest', 'enabled', 'deleted', 'affectedResources']) || !validRevision(v.revision) ||
      typeof v.affectedResources !== 'number' || !Number.isSafeInteger(v.affectedResources) || v.affectedResources < 1 || v.affectedResources > 4_294_967_295 ||
      !(v.enabled === null || typeof v.enabled === 'boolean')) return invalid();
  const deleting = intent.operation === 'delete';
  if (deleting ? typeof v.deleted !== 'boolean' || v.recordDigest !== null || v.enabled !== null :
    v.deleted !== null || !digest(v.recordDigest) || saving(intent.operation) && v.recordDigest !== intent.recordDigest ||
    intent.operation === 'enable' && v.enabled !== true || intent.operation === 'disable' && v.enabled !== false) return invalid();
  if (BigInt(v.revision) !== BigInt(intent.expectedRevision) + (deleting && v.deleted === false ? 0n : 1n) ||
      (intent.operation !== 'delete' || intent.kind !== 'action' || v.deleted === false) && v.affectedResources !== 1) return invalid();
  return v as unknown as DefinitionReceipt;
}
export function parseDefinitionState(v: unknown): DefinitionState {
  if (!exact(v, ['hostId', 'hostInstanceId', 'pending', 'recent']) || !text(v.hostId) || !text(v.hostInstanceId) || !Array.isArray(v.recent) || v.recent.length > 16) return invalid();
  const pending = v.pending === null ? null : parseIntent(v.pending);
  const recent = v.recent.map((entry): DefinitionRecord => {
    if (!exact(entry, ['intent', 'outcome', 'receipt', 'errorCode', 'recordedAt']) ||
        typeof entry.outcome !== 'string' || !['succeeded', 'rejected', 'reviewed-unknown'].includes(entry.outcome) || !timestamp(entry.recordedAt)) return invalid();
    const intent = parseIntent(entry.intent);
    if (entry.outcome === 'succeeded') {
      if (entry.errorCode !== null) return invalid();
      return { intent, outcome: entry.outcome, receipt: parseReceipt(entry.receipt, intent), errorCode: null, recordedAt: entry.recordedAt };
    }
    if (entry.receipt !== null || (entry.outcome === 'rejected' ? !text(entry.errorCode, 128) || !rejected.includes(entry.errorCode) : entry.errorCode !== null)) return invalid();
    return { intent, outcome: entry.outcome as 'rejected' | 'reviewed-unknown', receipt: null, errorCode: entry.errorCode as string | null, recordedAt: entry.recordedAt };
  });
  const ids = recent.map((r) => r.intent.commandId);
  if (new Set(ids).size !== ids.length || pending && ids.includes(pending.commandId)) return invalid();
  return { hostId: v.hostId, hostInstanceId: v.hostInstanceId, pending, recent };
}

/** Fresh acknowledgement is linked to a single successful native receipt. It
 * is never synthesized from current source or returned by historical recovery. */
export function parseDefinitionSubmission(v: unknown, input: DefinitionWrite, host: { hostId: string; hostInstanceId: string }): DefinitionSubmission {
  if (!exact(v, ['state', 'acknowledgement'])) return invalid();
  const state = parseDefinitionState(v.state);
  if (state.hostId !== host.hostId || state.hostInstanceId !== host.hostInstanceId) return invalid();
  if (v.acknowledgement === null) return { state, acknowledgement: null };
  const a = v.acknowledgement;
  if (!exact(a, ['value', 'resourceVersions', 'command', 'hostInstanceId']) || a.hostInstanceId !== host.hostInstanceId || state.pending ||
      !exact(a.command, ['commandId', 'state', 'resource', 'committedAt']) || a.command.state !== 'succeeded' || !timestamp(a.command.committedAt) || a.command.committedAt === 0 ||
      !exact(a.command.resource, ['kind', 'scope', 'id']) || a.command.resource.kind !== 'automation' || a.command.resource.scope !== input.kind || a.command.resource.id !== input.id ||
      !Array.isArray(a.resourceVersions) || !a.resourceVersions.length) return invalid();
  const commandId = a.command.commandId;
  const result = state.recent[0];
  if (!result || result.intent.commandId !== commandId || result.outcome !== 'succeeded' || !result.receipt || result.intent.kind !== input.kind || result.intent.id !== input.id ||
      result.intent.operation !== input.operation || result.intent.expectedRevision !== input.expectedRevision || result.intent.hostInstanceId !== host.hostInstanceId ||
      a.resourceVersions.length !== result.receipt.affectedResources) return invalid();
  let previous = '';
  const ids = new Set<string>();
  for (const [position, version] of a.resourceVersions.entries()) {
    if (!exact(version, ['kind', 'scope', 'id', 'revision']) || version.kind !== 'automation' || !id(version.id) || !validRevision(version.revision) ||
        version.revision === '0' && !(position === 0 && input.operation === 'delete' && result.receipt.deleted === false)) return invalid();
    if (position === 0) {
      if (version.scope !== input.kind || version.id !== input.id || version.revision !== result.receipt.revision) return invalid();
    } else if (input.operation !== 'delete' || input.kind !== 'action' || version.scope !== 'event' || previous && utf8Compare(version.id, previous) <= 0) return invalid();
    const key = `${version.scope}:${version.id}`;
    if (ids.has(key)) return invalid();
    ids.add(key); previous = position ? version.id : '';
  }
  if (input.operation === 'delete') {
    if (!exact(a.value, ['id', 'deleted']) || a.value.id !== input.id || a.value.deleted !== result.receipt.deleted) return invalid();
  } else {
    if (saving(input.operation) && !sourceNumbersSafe(a.value)) throw new ControlCallError('source_precision', 'The saved source contains integers this browser cannot display exactly.');
    assertRecord(a.value as AutomationRecord, input.kind, input.id);
    if ('record' in input) {
      // This is the same normalized value the host hashed before persisting
      // its intent. Comparing values needs no browser cryptography and never
      // borrows a later current-record read as the acknowledgement.
      const expected = JSON.parse(JSON.stringify(input.record)) as Record<string, unknown>;
      if (input.operation === 'create' && (typeof expected.name !== 'string' || /^\p{White_Space}*$/u.test(expected.name))) {
        expected.name = input.kind === 'action' ? 'Untitled action' : 'Untitled event';
      }
      if (stableJson(a.value) !== stableJson(expected)) return invalid();
    }
    if ((input.operation === 'enable' || input.operation === 'disable') && (a.value as AutomationRecord).enabled !== result.receipt.enabled) return invalid();
  }
  return { state, acknowledgement: a as unknown as DefinitionAcknowledgement };
}

/** Only the shared pending/host boundary is needed before a definition write;
 * adoption/approval features independently validate their full metadata. */
export function parsePeerState(v: unknown, prefix: 'owner-adopt-' | 'owner-approval-'): { hostId: string; hostInstanceId: string; pending: boolean } {
  if (!exact(v, ['hostId', 'hostInstanceId', 'pending', 'recent']) || !text(v.hostId) || !text(v.hostInstanceId) || !Array.isArray(v.recent) || v.recent.length > 16 ||
      v.pending !== null && (!object(v.pending) || typeof v.pending.commandId !== 'string' || !v.pending.commandId.startsWith(prefix))) return invalid();
  return { hostId: v.hostId, hostInstanceId: v.hostInstanceId, pending: v.pending !== null };
}
