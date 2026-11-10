import { computed, ref, shallowRef, watch } from 'vue';
import { ControlCallError, errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { ownerMutationGate } from '@/shared/api/owner-mutation-gate.ts';
import { parseDefinitionState, parsePeerState } from '@/entities/automation/model/definition-contracts.ts';
import { checkLibraryMutation, libraryTarget, parseLibrarySnapshot, parseLibraryState, parseLibrarySubmission, sameLibraryHost,
  type LibraryAcknowledgement, type LibraryCloudProvenance, type LibraryMutation, type LibrarySnapshot, type LibraryState } from './library-contracts.ts';

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
export interface LibraryDraft {
  token: number; generation: number; mutation: LibraryMutation; context: LibrarySnapshot; busy: boolean; submitted: boolean; reviewRequired: boolean; error: string; provenance?: LibraryCloudProvenance | null;
}
function createLibrary(control: ControlClient) {
  const gate = ownerMutationGate(control);
  const state = shallowRef<LibraryState | null>(null);
  const snapshot = shallowRef<LibrarySnapshot | null>(null);
  const busy = ref(false); const unavailable = ref(true); const error = ref(''); const recoveryEpoch = ref(0);
  const draft = shallowRef<LibraryDraft | null>(null); const comparison = shallowRef<LibrarySnapshot | null>(null); const reading = ref(false);
  let refreshing: Promise<void> | null = null; let token = 0;
  let snapshotEpoch = 0; let installedEpoch = 0;
  let stateEpoch = 0;
  const snapshots = new WeakMap<LibrarySnapshot, number>();
  let completion: { resolve: () => void; reject: (failure: unknown) => void } | null = null;
  const sameFence = (a: LibrarySnapshot, b: LibrarySnapshot) => sameLibraryHost(a, b) && a.revision === b.revision && a.definitionDigest === b.definitionDigest;
  function invalidate(): void { if (draft.value && !draft.value.busy) draft.value = { ...draft.value, reviewRequired: true }; comparison.value = null; }
  watch(gate.owner, (owner) => { if (owner && owner !== 'library') invalidate(); }, { flush: 'sync' });
  async function fetchState(): Promise<LibraryState> {
    const epoch = ++stateEpoch;
    try { const next = parseLibraryState(await control.call('automation.library.get', {})); if (epoch === stateEpoch) { state.value = next; unavailable.value = false; } return next; }
    catch (failure) { unavailable.value = true; throw failure; }
  }
  async function readSnapshot(): Promise<LibrarySnapshot> {
    const epoch = ++snapshotEpoch;
    const next = parseLibrarySnapshot(await control.call('automation.library.snapshot', {})); snapshots.set(next, epoch); return next;
  }
  function installSnapshot(next: LibrarySnapshot): void {
    const epoch = snapshots.get(next) ?? ++snapshotEpoch;
    if (epoch < installedEpoch || snapshot.value && sameLibraryHost(snapshot.value, next) && BigInt(next.revision) < BigInt(snapshot.value.revision)) return;
    installedEpoch = epoch;
    snapshot.value = next;
    if (draft.value && !sameFence(draft.value.context, next)) invalidate();
  }
  async function refreshSnapshot(): Promise<LibrarySnapshot> { const next = await readSnapshot(); installSnapshot(next); return next; }
  function refresh(): Promise<void> {
    if (refreshing) return refreshing;
    if (busy.value) return Promise.resolve();
    busy.value = true; error.value = '';
    refreshing = (async () => {
      try { await fetchState(); } catch (failure) { error.value = errorMessage(failure); }
      finally { busy.value = false; refreshing = null; }
    })();
    return refreshing;
  }
  async function submit(mutation: LibraryMutation, context: LibrarySnapshot, provenance?: LibraryCloudProvenance | null): Promise<LibraryAcknowledgement> {
    if (!context?.hostId || !context.hostInstanceId) throw new ControlCallError('capability_unavailable', 'Review the current library before changing it.');
    const captured = clone(context); const candidate = checkLibraryMutation(mutation);
    if (refreshing) await refreshing;
    if (busy.value) throw new ControlCallError('command_pending', 'An Owner library operation is in progress.');
    return gate.run('library', async () => {
      busy.value = true; error.value = '';
      try {
        const [host, definition, adoption, approval] = await Promise.all([
          fetchState(), control.call('automation.definition.get', {}).then(parseDefinitionState),
          control.call('automation.adoption.get', {}).then((value) => parsePeerState(value, 'owner-adopt-')),
          control.call('automation.approval.get', {}).then((value) => parsePeerState(value, 'owner-approval-')),
        ]);
        if ([host, definition, adoption, approval].some((other) => !sameLibraryHost(captured, other))) throw new ControlCallError('host_restarted', 'The reviewed library belongs to a different host instance.');
        if (host.pending || definition.pending || adoption.pending || approval.pending) throw new ControlCallError('command_pending', 'Resolve the pending Owner operation before another change.');
        // Metadata preflight never refreshes or replaces the displayed source fence.
        const result = parseLibrarySubmission(await control.call('automation.library.submit', { mutation: candidate,
          expectedRevision: captured.revision, definitionDigest: captured.definitionDigest, hostId: captured.hostId, hostInstanceId: captured.hostInstanceId,
          ...(provenance ? { provenance } : {}) }), candidate, captured);
        ++stateEpoch; state.value = result.state; unavailable.value = false;
        if (!result.acknowledgement) {
          const last = result.state.recent[0];
          const rejected = last?.outcome === 'rejected' && last.intent.operation === candidate.operation && last.intent.target === libraryTarget(candidate) &&
            last.intent.expectedRevision === captured.revision && last.intent.definitionDigest === captured.definitionDigest ? last.errorCode : null;
          throw new ControlCallError(rejected ?? 'unconfirmed', rejected ? `Library change rejected (${rejected}). Review the current library.` : 'The original library outcome requires review.');
        }
        return result.acknowledgement;
      } catch (failure) {
        error.value = errorMessage(failure);
        try { await fetchState(); } catch { unavailable.value = true; }
        throw failure;
      } finally { busy.value = false; }
    });
  }
  function request(mutation: LibraryMutation, context: LibrarySnapshot | null = snapshot.value, provenance?: LibraryCloudProvenance | null): Promise<void> {
    if (!context) return Promise.reject(new ControlCallError('capability_unavailable', 'Read the current library before changing it.'));
    if (draft.value || gate.busy.value) return Promise.reject(new ControlCallError('command_pending', 'Finish the reviewed library operation first.'));
    let candidate: LibraryMutation;
    try { candidate = checkLibraryMutation(mutation); } catch (failure) { return Promise.reject(failure); }
    draft.value = { token: ++token, generation: 0, mutation: candidate, context: clone(context), busy: false, submitted: false, reviewRequired: false, error: '', provenance: provenance ?? null }; comparison.value = null;
    return new Promise<void>((resolve, reject) => { completion = { resolve, reject }; });
  }
  async function confirm(): Promise<void> {
    const captured = draft.value;
    if (!captured || captured.busy || captured.submitted || captured.reviewRequired || gate.busy.value) return;
    try { checkLibraryMutation(captured.mutation); } catch (failure) { draft.value = { ...captured, error: errorMessage(failure) }; return; }
    draft.value = { ...captured, busy: true, submitted: true, error: '' };
    try {
      await submit(captured.mutation, captured.context, captured.provenance ?? null);
      if (draft.value?.token !== captured.token) return;
      // A fresh acknowledgement is metadata only. Display comes from a new
      // coherent snapshot, never from the candidate or historical receipt.
      // Keep this review busy until that read completes: the next operation
      // must not borrow the catalog displayed before this known commit.
      busy.value = true;
      try { await refreshSnapshot(); } catch (failure) { error.value = errorMessage(failure); }
      finally { busy.value = false; }
      if (draft.value?.token !== captured.token) return;
      draft.value = draft.value.generation === captured.generation ? null : { ...draft.value, busy: false, reviewRequired: true, error: '' };
      comparison.value = null; const pending = completion; completion = null;
      pending?.resolve();
    } catch (failure) {
      if (draft.value?.token === captured.token) draft.value = { ...draft.value, busy: false, reviewRequired: true, error: errorMessage(failure) };
    }
  }
  function renameRecord(kind: 'action' | 'event', id: string, name: string): void {
    const current = draft.value;
    if (!current || !['apply-template', 'import-profile'].includes(current.mutation.operation)) return;
    const rename = (entry: { actions: import('../../../../automation/behavior/types.ts').LiveAction[]; event: import('../../../../automation/behavior/types.ts').LiveEvent }) => ({
      actions: kind === 'action' ? entry.actions.map((record) => record.id === id ? { ...record, name } : record) : entry.actions,
      event: kind === 'event' && entry.event.id === id ? { ...entry.event, name } : entry.event,
    });
    const mutation = current.mutation.operation === 'apply-template' ? { ...current.mutation, entry: rename(current.mutation.entry),
      ...(current.mutation.additionalEntries ? { additionalEntries: current.mutation.additionalEntries.map(rename) } : {}) } : current.mutation.operation === 'import-profile' ?
      { ...current.mutation, entries: current.mutation.entries.map(rename) } : current.mutation;
    draft.value = { ...current, mutation, generation: current.generation + 1 };
  }
  async function readForReview(): Promise<void> {
    const captured = draft.value; if (!captured || captured.busy || reading.value) return;
    reading.value = true; comparison.value = null;
    try { const current = await readSnapshot(); if (draft.value?.token === captured.token) comparison.value = current; }
    catch (failure) { if (draft.value?.token === captured.token) draft.value = { ...draft.value, error: errorMessage(failure) }; }
    finally { reading.value = false; }
  }
  function useReviewed(): void {
    if (!draft.value || !comparison.value || draft.value.busy || draft.value.submitted) return;
    const context = comparison.value; draft.value = { ...draft.value, context: clone(context), reviewRequired: false, error: '' }; comparison.value = null;
    installSnapshot(context);
  }
  function dismiss(): void {
    if (!draft.value || draft.value.busy) return;
    const submitted = draft.value.submitted;
    const message = draft.value.error;
    draft.value = null; comparison.value = null; const pending = completion; completion = null;
    pending?.reject(new ControlCallError(submitted ? 'unconfirmed' : 'cancelled', submitted ? message || 'Original library outcome must be reviewed; no change was resent.' : 'Library change cancelled.'));
  }
  async function recover(method: 'automation.library.resolve' | 'automation.library.review_unknown', params: Record<string, unknown>, consent: () => boolean = () => true): Promise<void> {
    if (busy.value || gate.busy.value) return;
    const captured = state.value;
    await gate.run('library', async () => {
      busy.value = true; error.value = '';
      try {
        const current = await fetchState();
        if (!captured || !sameLibraryHost(captured, current) || method.endsWith('review_unknown') && current.pending?.commandId !== params.commandId) throw new ControlCallError('stale_view', 'The original library operation changed.');
        if (!consent()) return;
        const next = parseLibraryState(await control.call(method, params));
        if (!sameLibraryHost(current, next)) throw new ControlCallError('host_restarted', 'The Owner host changed.');
        ++stateEpoch; state.value = next; unavailable.value = false;
      } catch (failure) { error.value = errorMessage(failure); try { await fetchState(); } catch { unavailable.value = true; } }
      finally { busy.value = false; ++recoveryEpoch.value; invalidate(); }
    });
  }
  return { gate, state, snapshot, busy, unavailable, error, recoveryEpoch, draft, comparison, reading,
    working: computed(() => !!draft.value), fetchState, refresh, readSnapshot, refreshSnapshot, installSnapshot,
    submit, request, confirm, dismiss, readForReview, useReviewed, renameRecord,
    resolve: () => recover('automation.library.resolve', {}), reviewUnknown: (commandId: string, consent?: () => boolean) => recover('automation.library.review_unknown', { commandId, acknowledgeUnknown: true }, consent) };
}
export type LibraryJournal = ReturnType<typeof createLibrary>;
const journals = new WeakMap<ControlClient, LibraryJournal>();
export function libraryJournal(control: ControlClient): LibraryJournal {
  let journal = journals.get(control); if (!journal) { journal = createLibrary(control); journals.set(control, journal); } return journal;
}
