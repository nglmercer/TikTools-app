import { ref, shallowRef } from 'vue';
import { ControlCallError, errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { ownerMutationGate } from '@/shared/api/owner-mutation-gate.ts';
import { sourceNumbersSafe } from '@/shared/lib/json-source.ts';
import { validRevision, type AutomationHost } from './record-versions.ts';
import { cloneRecord } from '../lib/automation-records.ts';
import { parseLibraryState } from '@/entities/rule-template/model/library-contracts.ts';
import { parseDefinitionState, parseDefinitionSubmission, parsePeerState, type DefinitionAcknowledgement, type DefinitionState, type DefinitionWrite } from './definition-contracts.ts';

const sameHost = (a: { hostId: string; hostInstanceId: string }, b: { hostId: string; hostInstanceId: string }) => a.hostId === b.hostId && a.hostInstanceId === b.hostInstanceId;
function createJournal(control: ControlClient) {
  const gate = ownerMutationGate(control);
  const state = shallowRef<DefinitionState | null>(null);
  const busy = ref(false);
  const unavailable = ref(true);
  const error = ref('');
  const recoveryEpoch = ref(0);
  let refreshing: Promise<void> | null = null;

  async function fetchState(): Promise<DefinitionState> {
    try {
      const next = parseDefinitionState(await control.call('automation.definition.get', {}));
      state.value = next; unavailable.value = false;
      return next;
    } catch (failure) { unavailable.value = true; throw failure; }
  }
  function refresh(): Promise<void> {
    if (refreshing) return refreshing;
    if (busy.value) return Promise.resolve();
    busy.value = true; error.value = '';
    refreshing = (async () => {
      try { await fetchState(); }
      catch (failure) { error.value = errorMessage(failure); }
      finally { busy.value = false; refreshing = null; }
    })();
    return refreshing;
  }
  async function recover(method: 'automation.definition.resolve' | 'automation.definition.review_unknown', params: Record<string, unknown>, consent: () => boolean = () => true): Promise<void> {
    if (busy.value || gate.busy.value) return;
    await gate.run('definition', async () => {
      busy.value = true; error.value = '';
      const captured = state.value;
      try {
        const current = await fetchState();
        if (!captured || !sameHost(captured, current) ||
            method === 'automation.definition.review_unknown' && current.pending?.commandId !== params.commandId) {
          throw new ControlCallError('stale_view', 'The pending Owner operation or host changed.');
        }
        if (!consent()) return;
        state.value = parseDefinitionState(await control.call(method, params));
        if (!sameHost(current, state.value)) throw new ControlCallError('host_restarted', 'The Owner host changed.');
        unavailable.value = false;
      } catch (failure) {
        error.value = errorMessage(failure);
        try { await fetchState(); } catch { unavailable.value = true; }
      } finally {
        // Only metadata changed. Editors must compare current source explicitly;
        // historical success must never install a body or advance a draft base.
        ++recoveryEpoch.value; busy.value = false;
      }
    });
  }
  async function submit(input: DefinitionWrite, reviewedHost: AutomationHost): Promise<DefinitionAcknowledgement> {
    if (!reviewedHost?.hostId || !reviewedHost.hostInstanceId) {
      throw new ControlCallError('capability_unavailable', 'Read and review the Owner host identity before writing.');
    }
    if (!validRevision(input.expectedRevision)) {
      throw new ControlCallError('invalid_params', 'Read the current definition revision before writing.');
    }
    if ('record' in input && !sourceNumbersSafe(input.record)) {
      throw new ControlCallError('unsafe_source', 'This browser cannot write source containing integers outside its exact range.');
    }
    // Capture the submitted source before any host read can yield to another edit.
    const capturedInput = cloneRecord(input);
    const captured = { hostId: reviewedHost.hostId, hostInstanceId: reviewedHost.hostInstanceId };
    if (refreshing) await refreshing;
    if (busy.value) throw new ControlCallError('command_pending', 'The Owner definition journal is busy.');
    return gate.run('definition', async () => {
      busy.value = true; error.value = '';
      try {
        const [host, adoption, approval, library] = await Promise.all([
          fetchState(),
          control.call('automation.adoption.get', {}).then((v) => parsePeerState(v, 'owner-adopt-')),
          control.call('automation.approval.get', {}).then((v) => parsePeerState(v, 'owner-approval-')),
          control.call('automation.library.get', {}).then(parseLibraryState),
        ]);
        if (!sameHost(captured, host) || !sameHost(host, adoption) || !sameHost(host, approval) || !sameHost(host, library)) {
          throw new ControlCallError('host_restarted', 'The Owner host changed. Read and review the current definition.');
        }
        if (host.pending || adoption.pending || approval.pending || library.pending) {
          throw new ControlCallError('command_pending', 'Resolve or review the pending Owner operation first.');
        }
        const { id, ...rest } = capturedInput;
        const params = { ...rest, ...(capturedInput.operation === 'create' ? {} : { id }), hostId: host.hostId, hostInstanceId: host.hostInstanceId };
        const next = parseDefinitionSubmission(await control.call('automation.definition.submit', params), capturedInput, host);
        state.value = next.state; unavailable.value = false;
        if (!next.acknowledgement) {
          const latest = next.state.recent[0];
          const rejected = latest?.intent.kind === capturedInput.kind && latest.intent.id === capturedInput.id &&
            latest.intent.operation === capturedInput.operation && latest.intent.expectedRevision === capturedInput.expectedRevision && latest.outcome === 'rejected' ? latest : null;
          throw new ControlCallError(rejected?.errorCode ?? 'unconfirmed', rejected?.errorCode
            ? `The Owner definition change was rejected (${rejected.errorCode}). Compare the current saved source.`
            : 'The original Owner definition outcome requires review.');
        }
        return next.acknowledgement;
      } catch (failure) {
        error.value = errorMessage(failure);
        // The native task owns its identity and may have committed. Read once;
        // never retry submit, replace a target ID or synthesize an acknowledgement.
        try { await fetchState(); } catch { unavailable.value = true; }
        throw failure;
      } finally { busy.value = false; }
    });
  }
  return { state, busy, unavailable, error, gate, recoveryEpoch, fetchState, refresh, submit,
    resolve: () => recover('automation.definition.resolve', {}),
    reviewUnknown: (commandId: string, consent?: () => boolean) => recover('automation.definition.review_unknown', { commandId, acknowledgeUnknown: true }, consent) };
}
export type DefinitionJournal = ReturnType<typeof createJournal>;
const journals = new WeakMap<ControlClient, DefinitionJournal>();
export function definitionJournal(control: ControlClient): DefinitionJournal {
  let journal = journals.get(control);
  if (!journal) { journal = createJournal(control); journals.set(control, journal); }
  return journal;
}
