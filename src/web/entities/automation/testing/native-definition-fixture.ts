import { ControlCallError, type ControlCallOptions, type ResourceVersion, type RpcParams } from '@/shared/api/control-client.ts';
import type { DefinitionIntent, DefinitionState } from '../model/definition-contracts.ts';

const copy = <T>(value: T): T => structuredClone(value);
/** Opt-in native protocol fixture around an existing in-memory business host. */
export function nativeDefinitionHost(business: (method: string, params: RpcParams | undefined, options?: ControlCallOptions) => unknown) {
  let state: DefinitionState = { hostId: 'host-a', hostInstanceId: 'instance-a', pending: null, recent: [] };
  let serial = 0;
  let loseReply = false;
  const transitions = { adoptionPending: false, approvalPending: false };
  async function handle(method: string, params?: RpcParams, options?: ControlCallOptions): Promise<unknown> {
    if (method === 'automation.definition.get') return copy(state);
    if (method === 'automation.adoption.get' || method === 'automation.approval.get') {
      const adoption = method === 'automation.adoption.get';
      return { hostId: state.hostId, hostInstanceId: state.hostInstanceId,
        pending: transitions[adoption ? 'adoptionPending' : 'approvalPending'] ? { commandId: `${adoption ? 'owner-adopt-' : 'owner-approval-'}${'f'.repeat(32)}` } : null, recent: [] };
    }
    if (method === 'automation.definition.resolve') return copy(state);
    if (method === 'automation.definition.review_unknown') {
      if (state.pending && state.pending.commandId === params?.commandId && params.acknowledgeUnknown === true) {
        state.recent.unshift({ intent: state.pending, outcome: 'reviewed-unknown', receipt: null, errorCode: null, recordedAt: 2 }); state.pending = null;
      }
      return copy(state);
    }
    if (method !== 'automation.definition.submit') return business(method, params, options);
    if (!params || state.pending || params.hostId !== state.hostId || params.hostInstanceId !== state.hostInstanceId) throw new ControlCallError('command_pending', 'native pending');
    const operation = params.operation as DefinitionIntent['operation'];
    const record = params.record as Record<string, unknown> | undefined;
    const id = operation === 'create' ? String(record?.id) : String(params.id);
    const kind = params.kind as DefinitionIntent['kind'];
    const intent: DefinitionIntent = { commandId: `owner-definition-${(++serial).toString(16).padStart(32, '0')}`, operation, kind, id,
      expectedRevision: String(params.expectedRevision), hostInstanceId: state.hostInstanceId,
      recordDigest: operation === 'create' || operation === 'update' ? 'a'.repeat(64) : null, createdAt: 1 };
    state.pending = intent;
    let versions: ResourceVersion[] = [];
    const businessParams = operation === 'create' ? { kind, record } : operation === 'update' ? { kind, id, record } : { kind, id };
    let value: unknown;
    try {
      value = await business(`automation.${operation}`, businessParams, { meta: { expectedRevision: intent.expectedRevision },
        onMetadata: (meta) => { versions = copy(meta?.resourceVersions ?? []); } });
    } catch (failure) {
      // Old generic UI fixtures use the adapter's read spelling; the native
      // coordinator calls the business command path, whose code is not_found.
      const code = failure instanceof ControlCallError ? failure.code === 'automation_not_found' ? 'not_found' : failure.code : '';
      if (!['revision_conflict', 'not_found', 'invalid_params', 'forbidden', 'capability_unavailable', 'host_restarted', 'too_large'].includes(code)) throw failure;
      state.recent.unshift({ intent, outcome: 'rejected', receipt: null, errorCode: code, recordedAt: 2 }); state.pending = null;
      return { state: copy(state), acknowledgement: null };
    }
    const body = value as Record<string, unknown>;
    if (!versions.length) throw new ControlCallError('definition_journal_invalid', 'The original outcome could not be verified.');
    state.recent.unshift({ intent, outcome: 'succeeded', receipt: { revision: versions[0]!.revision,
      recordDigest: operation === 'delete' ? null : intent.recordDigest ?? 'b'.repeat(64),
      enabled: operation === 'delete' ? null : typeof body.enabled === 'boolean' ? body.enabled : null,
      deleted: operation === 'delete' ? body.deleted as boolean : null, affectedResources: versions.length }, errorCode: null, recordedAt: 2 });
    state.recent = state.recent.slice(0, 16); state.pending = null;
    if (loseReply) { loseReply = false; throw new ControlCallError('timeout', 'reply lost after commit'); }
    return { state: copy(state), acknowledgement: { value, resourceVersions: versions, hostInstanceId: state.hostInstanceId,
      command: { commandId: intent.commandId, state: 'succeeded', resource: { kind: 'automation', scope: kind, id }, committedAt: 2 } } };
  }
  return { handle, transitions, state: () => state, replaceState: (next: DefinitionState) => { state = copy(next); }, loseNextReply: () => { loseReply = true; } };
}
