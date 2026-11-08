import type { DefinitionState } from '@/entities/automation/model/definition-contracts.ts';
import type { ControlCallOptions, RpcParams, ResourceVersion } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ApprovalState, ApprovalIntent } from '../model/native-contracts.ts';
import type { ApprovalSubject, ExecutionManifest, ApprovalSummary, ApprovalEligibility } from '../model/contracts.ts';
import type { AdoptionState, ProposalReview } from '@/features/review-automation-proposal/model/contracts.ts';
export { copy, deferred } from '@/features/review-automation-proposal/testing/fixture.ts';
import { copy } from '@/features/review-automation-proposal/testing/fixture.ts';

export const nativeId = (digit = '1'): string => `owner-approval-${digit.repeat(32)}`;
export function approvalFixture() {
  const delegated = { authority: { kind: 'delegated' as const, principalId: 'moderator', sessionId: 'author-session' }, generation: '1' };
  const source: ProposalReview = {
    kind: 'event', id: 'same',
    record: { id: 'same', name: 'Saved delegated proposal', trigger: 'tiktok.chat', enabled: false, runMode: 'random', actionIds: ['same', 'disabled', 'owner'] },
    provenance: copy(delegated),
    dependencies: [
      { id: 'same', record: { id: 'same', name: 'Saved log', typeId: 'core.log', enabled: false, config: { message: '<script>privateSource()</script> {{event.user.uniqueId}}' } }, provenance: copy(delegated) },
      { id: 'disabled', record: { id: 'disabled', name: 'Disabled alternative', typeId: 'core.delay', enabled: false, config: { ms: 100 } }, provenance: copy(delegated) },
      { id: 'owner', record: { id: 'owner', name: 'Owner action', typeId: 'core.log', enabled: false, config: { message: 'owner literal' } }, provenance: { authority: { kind: 'owner' }, generation: '2' } },
    ],
  };
  const versions: ResourceVersion[] = [
    { kind: 'automation', scope: 'event', id: 'same', revision: '4' },
    { kind: 'automation', scope: 'action', id: 'same', revision: '2' },
    { kind: 'automation', scope: 'action', id: 'disabled', revision: '3' },
    { kind: 'automation', scope: 'action', id: 'owner', revision: '5' },
  ];
  const subjects: ApprovalSubject[] = [{ principalId: 'moderator', sessionId: 'author-session', grantRevision: '3', configurationRevision: '2',
    grant: { capabilities: ['automation.read', 'automation.edit', 'live.read', 'points.read', 'points.adjust'], resources: { viewer: ['alice'], automation: ['event:same', 'action:same', 'action:disabled', 'action:owner'] }, settingsFields: {}, actions: {}, maxPointsDelta: 5 } }];
  let approval: ApprovalSummary = { state: 'none', generation: null, policyVersion: null, manifestDigest: null, current: false };
  let eligibility: ApprovalEligibility = { eligible: true, reasonCode: null };
  let manifestAvailable = true;
  let state: ApprovalState = { hostId: 'host-a', hostInstanceId: 'instance-a', pending: null, recent: [] };
  let adoptionState: AdoptionState = { hostId: 'host-a', hostInstanceId: 'instance-a', pending: null, recent: [] };
  let definitionState: DefinitionState | null = null;
  let serial = 0;
  const overrides = new Map<string, (params: RpcParams | undefined, options: ControlCallOptions | undefined) => unknown>();
  const requests: Array<{ method: string; params: RpcParams | undefined; options?: ControlCallOptions }> = [];
  function manifest(): ExecutionManifest | null {
    if (!manifestAvailable) return null;
    const pin = subjects.map(({ principalId, sessionId, grantRevision, configurationRevision }) => ({ principalId, sessionId, grantRevision, configurationRevision }));
    const nodes = [{ kind: source.kind, id: source.id, record: source.record, provenance: source.provenance }, ...source.dependencies.map((d) => ({ ...d, kind: 'action' as const }))];
    const bindings = nodes.map((d, i) => ({ kind: d.kind, id: d.id, revision: versions[i]!.revision, exists: d.record !== null, contentDigest: d.record ? 'a'.repeat(64) : null, provenance: copy(d.provenance) }));
    return { policyVersion: 1, digestAlgorithm: 'sha256-canonical-json-v1', hostId: state.hostId, subjects: pin, primary: bindings[0]!, dependencies: bindings.slice(1),
      effects: source.dependencies.map((d) => ({ family: 'action', id: d.id, typeId: typeof d.record?.typeId === 'string' ? d.record.typeId : null,
        kind: d.record === null ? 'missing-definition' : d.record.typeId === 'core.delay' ? 'delay' : 'log', enabled: typeof d.record?.enabled === 'boolean' ? d.record.enabled : null,
        decision: d.record === null ? 'unsupported' : 'classified', renderFields: d.id === 'same' ? ['event.user.uniqueId'] : [], dynamicTarget: false,
        requiredCapabilities: d.id === 'same' ? ['live.read'] : [], reasons: d.record === null ? ['missing-dependency'] : [] })),
      reasons: ['approval-required', ...(source.dependencies.some((d) => !d.record) ? ['missing-dependency'] : [])], executionAuthorized: false, manifestDigest: 'd'.repeat(64) };
  }
  function review() {
    return copy({ hostId: state.hostId, hostInstanceId: state.hostInstanceId, kind: source.kind, id: source.id, source,
      manifest: manifest(), subjects: manifestAvailable ? subjects : [], eligibility, approval });
  }
  function handle(method: string, params: RpcParams | undefined, options?: ControlCallOptions): unknown {
    if (method === 'automation.library.get') return copy({ hostId: state.hostId, hostInstanceId: state.hostInstanceId, pending: null, recent: [] });
    if (method === 'automation.definition.get') return copy(definitionState ?? { hostId: state.hostId, hostInstanceId: state.hostInstanceId, pending: null, recent: [] });
    if (method === 'automation.approval.get' || method === 'automation.approval.resolve') return copy(state);
    if (method === 'automation.adoption.get') return copy(adoptionState);
    if (method === 'automation.approval.review') {
      if (!params || Object.keys(params).length !== 2 || !Object.hasOwn(params, 'kind') || !Object.hasOwn(params, 'id')) {
        throw new ControlCallError('invalid_params', 'approval review accepts only kind and id');
      }
      options?.onMetadata?.({ resourceVersions: copy(versions), hostInstanceId: state.hostInstanceId });
      return review();
    }
    if (method === 'automation.approval.submit') {
      const intent: ApprovalIntent = { commandId: nativeId(String(++serial)), operation: params!.operation as 'activate' | 'revoke', kind: params!.kind as 'event', id: String(params!.id),
        expectedRevision: String(params!.expectedRevision), hostInstanceId: String(params!.hostInstanceId), createdAt: 1 };
      const activated = intent.operation === 'activate';
      const receipt = { revision: (BigInt(intent.expectedRevision) + 1n).toString(), approvalGeneration: String(serial), policyVersion: activated ? 1 : null, manifestDigest: activated ? 'e'.repeat(64) : null };
      state = { ...state, pending: null, recent: [{ intent, outcome: 'succeeded', receipt, errorCode: null, recordedAt: 2 }, ...state.recent] };
      return copy(state);
    }
    if (method === 'automation.approval.review_unknown' && state.pending) {
      state = { ...state, pending: null, recent: [{ intent: state.pending, outcome: 'reviewed-unknown', receipt: null, errorCode: null, recordedAt: 3 }, ...state.recent] };
      return copy(state);
    }
    throw new Error(`Unexpected ${method}`);
  }
  const handler = (method: string, params: RpcParams | undefined, options?: ControlCallOptions) => {
    requests.push({ method, params, options });
    return overrides.has(method) ? overrides.get(method)!(params, options) : handle(method, params, options);
  };
  const fake = createFakeControlClient(handler);
  return { ...fake, source, versions, subjects, requests, overrides, handle, handler, review, manifest,
    projection: () => new Map(versions.filter((v) => v.revision !== '0').map((v) => [`${v.scope}:${v.id}`, v.revision])),
    get definitionState() { return definitionState; }, set definitionState(value: DefinitionState | null) { definitionState = value; },
    get state() { return state; }, set state(value: ApprovalState) { state = value; },
    get adoptionState() { return adoptionState; }, set adoptionState(value: AdoptionState) { adoptionState = value; },
    get approval() { return approval; }, set approval(value: ApprovalSummary) { approval = value; },
    get eligibility() { return eligibility; }, set eligibility(value: ApprovalEligibility) { eligibility = value; },
    get manifestAvailable() { return manifestAvailable; }, set manifestAvailable(value: boolean) { manifestAvailable = value; },
  };
}
