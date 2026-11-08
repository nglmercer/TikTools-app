import { ControlCallError, type ResourceVersion } from '@/shared/api/control-client.ts';
import { automationVersions, recordKey, validRevision, type AutomationKind } from '@/entities/automation/model/record-versions.ts';

export interface ProposalTarget { kind: AutomationKind; id: string }
export interface Provenance {
  authority: { kind: 'owner' } | { kind: 'delegated'; principalId: string; sessionId: string };
  generation: string;
}
export interface ReviewedDefinition { record: Record<string, unknown> | null; provenance: Provenance | null }
export interface ProposalReview extends ProposalTarget, ReviewedDefinition {
  dependencies: Array<ReviewedDefinition & { id: string }>;
}
export interface CapturedReview {
  source: ProposalReview;
  versions: ResourceVersion[];
  hostId: string;
  hostInstanceId: string;
  fingerprint: string;
}
export interface AdoptionIntent extends ProposalTarget {
  commandId: string;
  expectedRevision: string;
  hostInstanceId: string;
  createdAt: number;
}
export interface AdoptionRecord {
  intent: AdoptionIntent;
  outcome: 'succeeded' | 'rejected' | 'reviewed-unknown';
  revision: string | null;
  errorCode: string | null;
  recordedAt: number;
}
export interface AdoptionState {
  hostId: string;
  hostInstanceId: string;
  pending: AdoptionIntent | null;
  recent: AdoptionRecord[];
}
export interface AdoptionSubmission extends ProposalTarget {
  expectedRevision: string;
  hostId: string;
  hostInstanceId: string;
  reviewedVersions: ResourceVersion[];
  acknowledgeAuthority: true;
}

const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const text = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && v.length <= 256;
const id = (v: unknown): v is string => text(v) && v.trim() === v && new TextEncoder().encode(v).length <= 128 && !v.includes('*') &&
  ![...v].some((character) => { const code = character.codePointAt(0)!; return code < 32 || (code >= 127 && code <= 159); });
const timestamp = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const kind = (v: unknown): v is AutomationKind => v === 'action' || v === 'event';
const positive = (v: unknown): v is string => validRevision(v) && v !== '0';
const invalid = (): never => { throw new ControlCallError('protocol', 'Invalid automation review or adoption journal response.'); };

function provenance(v: unknown): Provenance | null {
  if (v === null) return null;
  if (!object(v) || !positive(v.generation) || !object(v.authority)) return invalid();
  const a = v.authority;
  if (a.kind === 'owner' && Object.keys(a).length === 1) return v as unknown as Provenance;
  if (a.kind === 'delegated' && text(a.principalId) && text(a.sessionId) && Object.keys(a).length === 3) return v as unknown as Provenance;
  return invalid();
}
function definition(v: unknown, expectedId: string): ReviewedDefinition {
  if (!object(v) || !id(expectedId)) return invalid();
  // This is raw saved-source inspection, not executable-schema approval.
  // Legacy/incomplete bodies stay visible without inventing missing fields.
  if (v.record !== null && (!object(v.record) || v.record.id !== expectedId)) return invalid();
  return { record: v.record as Record<string, unknown> | null, provenance: provenance(v.provenance) };
}

/** Review bodies and all revisions are one response, never the editor candidate. */
export function parseReview(result: unknown, meta: unknown, target: ProposalTarget): { source: ProposalReview; versions: ResourceVersion[]; fingerprint: string } {
  if (!object(result) || result.kind !== target.kind || result.id !== target.id || !Array.isArray(result.dependencies)) return invalid();
  const primary = definition(result, target.id);
  const dependencies: ProposalReview['dependencies'] = [];
  const ids = new Set<string>();
  for (const raw of result.dependencies) {
    if (!object(raw) || !id(raw.id) || ids.has(raw.id)) return invalid();
    ids.add(raw.id);
    dependencies.push({ id: raw.id, ...definition(raw, raw.id) });
  }
  const references = primary.record?.actionIds;
  if (target.kind === 'event' && references !== undefined && (!Array.isArray(references) || !references.every(id))) return invalid();
  const expectedIds = [...new Set(target.kind === 'event' && Array.isArray(references) ? references as string[] : [])];
  if (ids.size !== expectedIds.length || dependencies.some((node, i) => node.id !== expectedIds[i])) return invalid();
  const versionMap = automationVersions(meta);
  const orderedKeys = [recordKey(target.kind, target.id), ...dependencies.map((node) => recordKey('action', node.id))];
  const keys = new Set(orderedKeys);
  if (versionMap.size !== keys.size || [...keys].some((key) => !versionMap.has(key))) return invalid();
  const rawVersions = (meta as { resourceVersions: unknown[] }).resourceVersions;
  if (rawVersions.some((v, i) => !object(v) || Object.keys(v).length !== 4 ||
      !Object.keys(v).every((key) => ['kind', 'scope', 'id', 'revision'].includes(key)) || recordKey(v.scope as AutomationKind, String(v.id)) !== orderedKeys[i])) return invalid();
  for (const [family, resourceId, node] of [[target.kind, target.id, primary], ...dependencies.map((d) => ['action', d.id, d] as const)] as const) {
    const revision = versionMap.get(recordKey(family as AutomationKind, resourceId))!;
    if (node.record && revision === '0') return invalid();
  }
  const source: ProposalReview = { ...target, ...primary, dependencies };
  const versions = (meta as { resourceVersions: ResourceVersion[] }).resourceVersions.map((v) => ({ ...v }));
  return { source, versions, fingerprint: stableJson({ source, versions: [...versions].sort((a, b) => recordKey(a.scope as AutomationKind, a.id).localeCompare(recordKey(b.scope as AutomationKind, b.id))) }) };
}

function intent(v: unknown): AdoptionIntent {
  if (!object(v) || !text(v.commandId) || !kind(v.kind) || !id(v.id) || !validRevision(v.expectedRevision) || !text(v.hostInstanceId) || !timestamp(v.createdAt)) return invalid();
  return v as unknown as AdoptionIntent;
}
export function parseAdoptionState(v: unknown): AdoptionState {
  if (!object(v) || !text(v.hostId) || !text(v.hostInstanceId) || !Array.isArray(v.recent) || v.recent.length > 16) return invalid();
  const pending = v.pending === null ? null : intent(v.pending);
  const recent = v.recent.map((raw): AdoptionRecord => {
    if (!object(raw) || !['succeeded', 'rejected', 'reviewed-unknown'].includes(String(raw.outcome)) ||
        !(raw.revision === null || positive(raw.revision)) || !(raw.errorCode === null || text(raw.errorCode)) || !timestamp(raw.recordedAt)) return invalid();
    return { ...raw, intent: intent(raw.intent) } as unknown as AdoptionRecord;
  });
  if (new Set(recent.map((r) => r.intent.commandId)).size !== recent.length || recent.some((r) => r.intent.commandId === pending?.commandId)) return invalid();
  return { hostId: v.hostId, hostInstanceId: v.hostInstanceId, pending, recent };
}

export function stableJson(value: unknown): string {
  return JSON.stringify(value, (_key, node: unknown) => object(node) ? Object.fromEntries(Object.keys(node).sort().map((key) => [key, node[key]])) : node);
}
/** Rust preserves JSON integer values that JSON.parse can round. Such saved
 * source remains inspectable, but this browser cannot claim exact review. */
export function sourceNumbersDisplayExactly(source: unknown): boolean {
  const pending = [source];
  while (pending.length) {
    const value = pending.pop();
    if (typeof value === 'number' && (!Number.isFinite(value) || Number.isInteger(value) && !Number.isSafeInteger(value))) return false;
    if (value && typeof value === 'object') for (const child of Object.values(value)) pending.push(child);
  }
  return true;
}
export const sameHost = (a: { hostId: string; hostInstanceId: string }, b: { hostId: string; hostInstanceId: string }): boolean =>
  a.hostId === b.hostId && a.hostInstanceId === b.hostInstanceId;

/** The atomic owner review can be newer than the independently refreshed
 * library. Missing entries become deletion evidence only after that projection
 * has actually observed the reviewed identity at its revision or later. */
export function projectionChanged(review: CapturedReview, current: ReadonlyMap<string, string>, observedReviewed: Set<string>): boolean {
  return review.versions.some((version) => {
    const key = recordKey(version.scope as AutomationKind, version.id);
    const observed = current.get(key);
    if (observed !== undefined) {
      if (!validRevision(observed)) return true;
      const delta = BigInt(observed) - BigInt(version.revision);
      if (delta >= 0n) observedReviewed.add(key);
      return delta > 0n;
    }
    const node = version.scope === review.source.kind && version.id === review.source.id ? review.source
      : review.source.dependencies.find((d) => d.id === version.id);
    return !!node?.record && observedReviewed.has(key);
  });
}
