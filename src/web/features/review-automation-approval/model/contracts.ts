import type { Grant, Capability } from '@/entities/collaboration/api/types.ts';
import { capabilities } from '@/entities/collaboration/api/types.ts';
import { parseGrant } from '@/entities/collaboration/lib/grants.ts';
import type { AutomationKind } from '@/entities/automation/model/record-versions.ts';
import { parseReview, stableJson, type CapturedReview, type ProposalReview, type ProposalTarget, type Provenance } from '@/features/review-automation-proposal/model/contracts.ts';
import { digest, exact, invalid, object, positive, text } from './native-contracts.ts';

export interface ApprovalSubject {
  principalId: string; sessionId: string; grantRevision: string; configurationRevision: string; grant: Grant;
}
interface DefinitionBinding {
  kind: AutomationKind; id: string; revision: string; exists: boolean; contentDigest: string | null; provenance: Provenance | null;
}
export interface ManifestEffect {
  family: 'action'; id: string; typeId: string | null;
  kind: 'log' | 'emit' | 'points' | 'delay' | 'script' | 'http' | 'audio' | 'plugin' | 'unknown' | 'missing-definition';
  enabled: boolean | null; decision: 'classified' | 'review-required' | 'unsupported';
  renderFields: string[]; dynamicTarget: boolean; requiredCapabilities: Capability[]; reasons: string[];
}
export interface ExecutionManifest {
  policyVersion: number; digestAlgorithm: 'sha256-canonical-json-v1'; hostId: string;
  subjects: Array<Omit<ApprovalSubject, 'grant'>>; primary: DefinitionBinding; dependencies: DefinitionBinding[];
  effects: ManifestEffect[]; reasons: string[]; executionAuthorized: false; manifestDigest: string;
}
export interface ApprovalSummary {
  state: 'none' | 'active' | 'invalidated' | 'revoked'; generation: string | null;
  policyVersion: number | null; manifestDigest: string | null; current: boolean;
}
export interface ApprovalEligibility {
  eligible: boolean;
  reasonCode: null | 'definition-missing' | 'no-delegated-subjects' | 'subjects-unavailable' | 'policy-unavailable';
}
export interface CapturedApprovalReview extends CapturedReview {
  manifest: ExecutionManifest | null; subjects: ApprovalSubject[]; eligibility: ApprovalEligibility; approval: ApprovalSummary;
}
const reasons = [
  'approval-required', 'missing-dependency', 'missing-capability', 'resource-forbidden', 'live-restriction',
  'invalid-configuration', 'dynamic-target', 'global-access-requires-policy', 'unclassifiable-render-field',
  'opaque-script', 'effect-policy-unavailable', 'plugin-binding-unavailable', 'unknown-action-type',
];
function reasonList(v: unknown): v is string[] {
  return Array.isArray(v) && v.length <= reasons.length && new Set(v).size === v.length && v.every((r) => typeof r === 'string' && reasons.includes(r));
}
function binding(raw: unknown, expected: { kind: AutomationKind; id: string; revision: string; record: Record<string, unknown> | null; provenance: Provenance | null }): DefinitionBinding {
  if (!exact(raw, ['kind', 'id', 'revision', 'exists', 'contentDigest', 'provenance']) || raw.kind !== expected.kind || raw.id !== expected.id ||
      raw.revision !== expected.revision || raw.exists !== (expected.record !== null) ||
      (expected.record ? !digest(raw.contentDigest) : raw.contentDigest !== null) || stableJson(raw.provenance) !== stableJson(expected.provenance)) return invalid();
  return raw as unknown as DefinitionBinding;
}
function effect(raw: unknown, expected: { id: string; record: Record<string, unknown> | null }): ManifestEffect {
  if (!exact(raw, ['family', 'id', 'typeId', 'kind', 'enabled', 'decision', 'renderFields', 'dynamicTarget', 'requiredCapabilities', 'reasons']) ||
      raw.family !== 'action' || raw.id !== expected.id || !(raw.typeId === null || text(raw.typeId, 128)) ||
      typeof raw.kind !== 'string' || !['log', 'emit', 'points', 'delay', 'script', 'http', 'audio', 'plugin', 'unknown', 'missing-definition'].includes(raw.kind) ||
      !(raw.enabled === null || typeof raw.enabled === 'boolean') || typeof raw.decision !== 'string' || !['classified', 'review-required', 'unsupported'].includes(raw.decision) ||
      typeof raw.dynamicTarget !== 'boolean' || !Array.isArray(raw.renderFields) || raw.renderFields.length > 256 ||
      new Set(raw.renderFields).size !== raw.renderFields.length || raw.renderFields.some((f) => !text(f)) ||
      !Array.isArray(raw.requiredCapabilities) || new Set(raw.requiredCapabilities).size !== raw.requiredCapabilities.length ||
      raw.requiredCapabilities.some((c) => !capabilities.includes(c as Capability)) || !reasonList(raw.reasons)) return invalid();
  const typeId = expected.record?.typeId ?? expected.record?.type;
  if (raw.typeId !== (typeof typeId === 'string' ? typeId : null) || raw.enabled !== (typeof expected.record?.enabled === 'boolean' ? expected.record.enabled : null)) return invalid();
  return raw as unknown as ManifestEffect;
}
function parseSubjects(raw: unknown): ApprovalSubject[] {
  if (!Array.isArray(raw) || raw.length > 513) return invalid();
  const subjects = raw.map((s): ApprovalSubject => {
    if (!exact(s, ['principalId', 'sessionId', 'grantRevision', 'configurationRevision', 'grant']) ||
        !text(s.principalId) || !text(s.sessionId) || !positive(s.grantRevision) || !positive(s.configurationRevision) ||
        !(exact(s.grant, ['capabilities', 'resources', 'settingsFields', 'actions', 'maxPointsDelta']) ||
          exact(s.grant, ['capabilities', 'resources', 'settingsFields', 'actions', 'maxPointsDelta', 'effects']))) return invalid();
    let grant: Grant;
    try { grant = parseGrant(s.grant); } catch { return invalid(); }
    if (new Set(grant.capabilities).size !== grant.capabilities.length) return invalid();
    if (grant.effects && !grant.effects.http.length && !grant.effects.audio.length && !grant.effects.emitTopics.length) return invalid();
    return { principalId: s.principalId, sessionId: s.sessionId, grantRevision: s.grantRevision, configurationRevision: s.configurationRevision, grant };
  });
  if (new Set(subjects.map((s) => `${s.principalId}\0${s.sessionId}`)).size !== subjects.length) return invalid();
  return subjects;
}
function parseSummary(raw: unknown): ApprovalSummary {
  if (!exact(raw, ['state', 'generation', 'policyVersion', 'manifestDigest', 'current']) ||
      typeof raw.state !== 'string' || !['none', 'active', 'invalidated', 'revoked'].includes(raw.state) || typeof raw.current !== 'boolean') return invalid();
  if (raw.state === 'none') {
    if (raw.generation !== null || raw.policyVersion !== null || raw.manifestDigest !== null || raw.current) return invalid();
  } else if (!positive(raw.generation) || !Number.isSafeInteger(raw.policyVersion) || (raw.policyVersion as number) < 1 ||
      (raw.policyVersion as number) > 4_294_967_295 || !digest(raw.manifestDigest) || raw.current && raw.state !== 'active') return invalid();
  return raw as unknown as ApprovalSummary;
}

/** One saved-source/manifest/subject/status snapshot. No editor candidate or
 * independently refreshed effect description can replace this captured review. */
export function parseApprovalReview(raw: unknown, meta: unknown, target: ProposalTarget): CapturedApprovalReview {
  if (!exact(raw, ['hostId', 'hostInstanceId', 'kind', 'id', 'source', 'manifest', 'subjects', 'eligibility', 'approval']) ||
      raw.kind !== target.kind || raw.id !== target.id || !text(raw.hostId) || !text(raw.hostInstanceId) ||
      !exact(raw.source, ['kind', 'id', 'record', 'provenance', 'dependencies'])) return invalid();
  try { if (new TextEncoder().encode(JSON.stringify({ raw, meta })).length > 4 * 1024 * 1024) return invalid(); } catch { return invalid(); }
  const reviewed = parseReview(raw.source, meta, target);
  const source = reviewed.source;
  if (!Array.isArray(raw.source.dependencies) || source.dependencies.length > 512 || raw.source.dependencies.some((d) => !exact(d, ['id', 'record', 'provenance']))) return invalid();
  if ([source, ...source.dependencies].some((node) => node.provenance !== null && !exact(node.provenance, ['authority', 'generation']))) return invalid();
  if (!object(meta) || meta.command !== undefined || meta.hostInstanceId !== raw.hostInstanceId) return invalid();
  const subjects = parseSubjects(raw.subjects);
  const approval = parseSummary(raw.approval);
  if (!exact(raw.eligibility, ['eligible', 'reasonCode']) || typeof raw.eligibility.eligible !== 'boolean' ||
      !(raw.eligibility.reasonCode === null || typeof raw.eligibility.reasonCode === 'string' && ['definition-missing', 'no-delegated-subjects', 'subjects-unavailable', 'policy-unavailable'].includes(raw.eligibility.reasonCode)) ||
      raw.eligibility.eligible !== (raw.eligibility.reasonCode === null)) return invalid();
  const eligibility = raw.eligibility as unknown as ApprovalEligibility;
  let manifest: ExecutionManifest | null = null;
  if (raw.manifest !== null) {
    const m = raw.manifest;
    if (!exact(m, ['policyVersion', 'digestAlgorithm', 'hostId', 'subjects', 'primary', 'dependencies', 'effects', 'reasons', 'executionAuthorized', 'manifestDigest']) ||
        m.policyVersion !== 1 || m.digestAlgorithm !== 'sha256-canonical-json-v1' || m.hostId !== raw.hostId || m.executionAuthorized !== false || !digest(m.manifestDigest) ||
        !Array.isArray(m.subjects) || !Array.isArray(m.dependencies) || m.dependencies.length !== source.dependencies.length ||
        !Array.isArray(m.effects) || !reasonList(m.reasons)) return invalid();
    const pins = subjects.map(({ principalId, sessionId, grantRevision, configurationRevision }) => ({ principalId, sessionId, grantRevision, configurationRevision }));
    if (m.subjects.some((s) => !exact(s, ['principalId', 'sessionId', 'grantRevision', 'configurationRevision'])) || stableJson(m.subjects) !== stableJson(pins)) return invalid();
    const nodes = [{ kind: target.kind, id: target.id, record: source.record, provenance: source.provenance }, ...source.dependencies.map((node) => ({ ...node, kind: 'action' as const }))];
    const dependencies = m.dependencies;
    const rawEffects = m.effects;
    const bindings = nodes.map((node, index) => binding(index === 0 ? m.primary : dependencies[index - 1], { ...node, revision: reviewed.versions[index]!.revision }));
    const actions = target.kind === 'action' ? [{ id: source.id, record: source.record }] : source.dependencies;
    if (m.effects.length !== actions.length) return invalid();
    const effects = actions.map((node, index) => effect(rawEffects[index], node));
    const needed = [...new Set(nodes.flatMap((node) => node.provenance?.authority.kind === 'delegated' ? [`${node.provenance.authority.principalId}\0${node.provenance.authority.sessionId}`] : []))].sort();
    if (stableJson(subjects.map((s) => `${s.principalId}\0${s.sessionId}`).sort()) !== stableJson(needed)) return invalid();
    manifest = { policyVersion: 1, digestAlgorithm: 'sha256-canonical-json-v1', hostId: raw.hostId, subjects: pins,
      primary: bindings[0]!, dependencies: bindings.slice(1), effects, reasons: m.reasons, executionAuthorized: false, manifestDigest: m.manifestDigest };
  }
  if (eligibility.eligible && (!manifest || !source.record || subjects.length === 0 || manifest.reasons.some((r) => !['approval-required', 'dynamic-target'].includes(r)) ||
      manifest.effects.some((e) => !['core.log', 'core.delay', 'core.points', 'core.points.subtract'].includes(e.typeId ?? '') || e.decision === 'unsupported'))) return invalid();
  if (approval.current && (!eligibility.eligible || !manifest || approval.manifestDigest !== manifest.manifestDigest || approval.policyVersion !== manifest.policyVersion)) return invalid();
  return { source: source as ProposalReview, versions: reviewed.versions, hostId: raw.hostId, hostInstanceId: raw.hostInstanceId, manifest, subjects, eligibility, approval,
    fingerprint: stableJson({ source, versions: reviewed.versions, manifest, subjects, eligibility, approval, hostId: raw.hostId, hostInstanceId: raw.hostInstanceId }) };
}
