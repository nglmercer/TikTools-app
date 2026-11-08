import type { DefinitionState } from '@/entities/automation/model/definition-contracts.ts';
import type { ControlCallOptions, RpcParams, ResourceVersion } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { AdoptionIntent, AdoptionState, ProposalReview } from '../model/contracts.ts';
import type { ApprovalState } from '@/features/review-automation-approval/model/native-contracts.ts';

export const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
export function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
export function proposalFixture() {
  const delegated = { authority: { kind: 'delegated' as const, principalId: 'moderator', sessionId: 'author-session' }, generation: '1' };
  const owner = { authority: { kind: 'owner' as const }, generation: '2' };
  const source: ProposalReview = {
    kind: 'event', id: 'same',
    record: { id: 'same', name: 'Saved proposal', trigger: 'tiktok.chat', enabled: false, runMode: 'random', actionIds: ['same', 'disabled', 'missing'] },
    provenance: delegated,
    dependencies: [
      { id: 'same', record: { id: 'same', name: 'Script action', typeId: 'core.code', enabled: true, config: { source: '<script>privateSource()</script>' } }, provenance: delegated },
      { id: 'disabled', record: { id: 'disabled', name: 'Disabled guard', typeId: 'core.delay', enabled: false, config: { ms: 100 } }, provenance: owner },
      { id: 'missing', record: null, provenance: null },
    ],
  };
  const versions: ResourceVersion[] = [
    { kind: 'automation', scope: 'event', id: 'same', revision: '4' },
    { kind: 'automation', scope: 'action', id: 'same', revision: '2' },
    { kind: 'automation', scope: 'action', id: 'disabled', revision: '3' },
    { kind: 'automation', scope: 'action', id: 'missing', revision: '0' },
  ];
  let state: AdoptionState = { hostId: 'host-a', hostInstanceId: 'instance-a', pending: null, recent: [] };
  let approvalState: ApprovalState = { hostId: 'host-a', hostInstanceId: 'instance-a', pending: null, recent: [] };
  const overrides = new Map<string, (params: RpcParams | undefined, options: ControlCallOptions | undefined) => unknown>();
  const requests: Array<{ method: string; params: RpcParams | undefined; options: ControlCallOptions | undefined }> = [];
  let definitionState: DefinitionState | null = null;
  let serial = 0;
  function handle(method: string, params: RpcParams | undefined, options?: ControlCallOptions): unknown {
    if (method === 'automation.definition.get') return copy(definitionState ?? { hostId: state.hostId, hostInstanceId: state.hostInstanceId, pending: null, recent: [] });
    if (method === 'automation.adoption.get' || method === 'automation.adoption.resolve') return copy(state);
    if (method === 'automation.approval.get') return copy(approvalState);
    if (method === 'automation.review') {
      if (!params || Object.keys(params).length !== 2 || !Object.hasOwn(params, 'kind') || !Object.hasOwn(params, 'id')) {
        throw new ControlCallError('invalid_params', 'saved review accepts only kind and id');
      }
      options?.onMetadata?.({ resourceVersions: copy(versions) });
      return copy(source);
    }
    if (method === 'automation.adoption.submit') {
      const intent: AdoptionIntent = { kind: params!.kind as 'event', id: String(params!.id), commandId: `native-adoption-${++serial}`,
        expectedRevision: String(params!.expectedRevision), hostInstanceId: String(params!.hostInstanceId), createdAt: 1 };
      state = { ...state, pending: null, recent: [{ intent, outcome: 'succeeded', revision: '5', errorCode: null, recordedAt: 2 }, ...state.recent] };
      return copy(state);
    }
    if (method === 'automation.adoption.review_unknown' && state.pending) {
      state = { ...state, recent: [{ intent: state.pending, outcome: 'reviewed-unknown', revision: null, errorCode: null, recordedAt: 3 }, ...state.recent], pending: null };
      return copy(state);
    }
    throw new Error(`Unexpected ${method}`);
  }
  const handler = (method: string, params: RpcParams | undefined, options?: ControlCallOptions): unknown => {
    requests.push({ method, params, options });
    const override = overrides.get(method);
    return override ? override(params, options) : handle(method, params, options);
  };
  const fake = createFakeControlClient(handler);
  const projection = () => new Map(versions.filter((v) => v.revision !== '0').map((v) => [`${v.scope}:${v.id}`, v.revision]));
  return { ...fake, source, versions, requests, overrides, handle, handler, projection,
    get definitionState() { return definitionState; }, set definitionState(value: DefinitionState | null) { definitionState = value; },
    get state() { return state; }, set state(value: AdoptionState) { state = value; },
    get approvalState() { return approvalState; }, set approvalState(value: ApprovalState) { approvalState = value; } };
}
