import { ControlCallError, type ResourceVersion } from '@/shared/api/control-client.ts';
import { validRevision } from '@/entities/automation/model/record-versions.ts';
import type { ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';

export type ApprovalOperation = 'activate' | 'revoke';
export interface ApprovalEnablement extends ProposalTarget { enabled: boolean }
interface SubmissionTarget extends ProposalTarget {
  expectedRevision: string; hostId: string; hostInstanceId: string;
}
export type ApprovalSubmission = SubmissionTarget & (
  | { operation: 'activate'; reviewedVersions: ResourceVersion[]; manifestDigest: string; enablePlan: ApprovalEnablement[]; acknowledgeEffects: true }
  | { operation: 'revoke'; acknowledgeRevocation: true }
);
export interface ApprovalIntent extends ProposalTarget {
  commandId: string; operation: ApprovalOperation; expectedRevision: string;
  hostInstanceId: string; createdAt: number;
}
export interface ApprovalReceipt { revision: string; approvalGeneration: string; policyVersion: number | null; manifestDigest: string | null }
export interface ApprovalRecord {
  intent: ApprovalIntent; outcome: 'succeeded' | 'rejected' | 'reviewed-unknown';
  receipt: ApprovalReceipt | null; errorCode: string | null; recordedAt: number;
}
export interface ApprovalState { hostId: string; hostInstanceId: string; pending: ApprovalIntent | null; recent: ApprovalRecord[] }

export const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
export const exact = (v: unknown, keys: readonly string[]): v is Record<string, unknown> => object(v) && Object.keys(v).length === keys.length && keys.every((k) => Object.hasOwn(v, k));
export const text = (v: unknown, limit = 256): v is string => typeof v === 'string' && v.length > 0 && new TextEncoder().encode(v).length <= limit && v.trim() === v && !/\p{Cc}/u.test(v);
export const resourceId = (v: unknown): v is string => text(v, 128) && !v.includes('*');
export const positive = (v: unknown): v is string => validRevision(v) && v !== '0';
export const digest = (v: unknown): v is string => typeof v === 'string' && /^[0-9a-f]{64}$/.test(v);
export const invalid = (): never => { throw new ControlCallError('protocol', 'Invalid delegated approval review or native journal response.'); };
const timestamp = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const commandId = (v: unknown): v is string => typeof v === 'string' && /^owner-approval-[0-9a-f]{32}$/.test(v);
const rejectedCodes = ['revision_conflict', 'not_found', 'invalid_params', 'forbidden', 'capability_unavailable', 'host_restarted', 'too_large'];

function parseIntent(v: unknown): ApprovalIntent {
  if (!exact(v, ['commandId', 'operation', 'kind', 'id', 'expectedRevision', 'hostInstanceId', 'createdAt']) ||
    !commandId(v.commandId) || typeof v.operation !== 'string' || !['activate', 'revoke'].includes(v.operation) || typeof v.kind !== 'string' || !['action', 'event'].includes(v.kind) ||
    !resourceId(v.id) || !positive(v.expectedRevision) || !text(v.hostInstanceId) || !timestamp(v.createdAt)) return invalid();
  return v as unknown as ApprovalIntent;
}
function parseReceipt(v: unknown, intent: ApprovalIntent): ApprovalReceipt {
  if (!exact(v, ['revision', 'approvalGeneration', 'policyVersion', 'manifestDigest']) || !positive(v.revision) ||
      BigInt(v.revision) !== BigInt(intent.expectedRevision) + 1n || !positive(v.approvalGeneration) ||
      (intent.operation === 'activate' ? v.policyVersion !== 1 || !digest(v.manifestDigest) : v.policyVersion !== null || v.manifestDigest !== null)) return invalid();
  return v as unknown as ApprovalReceipt;
}

/** Native-owned metadata only. Source and an editable enable plan never live in
 * the journal response or browser persistence. Historical receipts are not permits. */
export function parseApprovalState(v: unknown): ApprovalState {
  if (!exact(v, ['hostId', 'hostInstanceId', 'pending', 'recent']) || !text(v.hostId) || !text(v.hostInstanceId) || !Array.isArray(v.recent) || v.recent.length > 16) return invalid();
  const pending = v.pending === null ? null : parseIntent(v.pending);
  const recent = v.recent.map((entry): ApprovalRecord => {
    if (!exact(entry, ['intent', 'outcome', 'receipt', 'errorCode', 'recordedAt']) ||
        typeof entry.outcome !== 'string' || !['succeeded', 'rejected', 'reviewed-unknown'].includes(entry.outcome) || !timestamp(entry.recordedAt)) return invalid();
    const intent = parseIntent(entry.intent);
    if (entry.outcome === 'succeeded') {
      if (entry.errorCode !== null || entry.receipt === null) return invalid();
      return { intent, outcome: entry.outcome, receipt: parseReceipt(entry.receipt, intent), errorCode: null, recordedAt: entry.recordedAt };
    }
    if (entry.receipt !== null || (entry.outcome === 'rejected' ? !text(entry.errorCode, 128) || !rejectedCodes.includes(entry.errorCode) : entry.errorCode !== null)) return invalid();
    return { intent, outcome: entry.outcome as 'rejected' | 'reviewed-unknown', receipt: null, errorCode: entry.errorCode as string | null, recordedAt: entry.recordedAt };
  });
  const ids = recent.map((r) => r.intent.commandId);
  if (new Set(ids).size !== ids.length || pending && ids.includes(pending.commandId)) return invalid();
  return { hostId: v.hostId, hostInstanceId: v.hostInstanceId, pending, recent };
}
