import { definitionJournal } from '@/entities/automation/model/definition-journal.ts';
import { ownerMutationGate } from '@/shared/api/owner-mutation-gate.ts';
import { computed, ref, shallowRef, watch } from 'vue';
import { callWithMeta, errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { recordKey } from '@/entities/automation/model/record-versions.ts';
import { adoptionJournal } from './adoption-journal.ts';
import { approvalJournal } from '@/features/review-automation-approval/model/approval-journal.ts';
import { parseReview, projectionChanged, sameHost, sourceNumbersDisplayExactly, stableJson, type AdoptionSubmission, type CapturedReview, type ProposalTarget } from './contracts.ts';

export function useProposalReview(control: ControlClient) {
  const journal = adoptionJournal(control);
  const approvals = approvalJournal(control);
  const definitions = definitionJournal(control);
  const gate = ownerMutationGate(control);
  const target = shallowRef<ProposalTarget | null>(null);
  const review = shallowRef<CapturedReview | null>(null);
  const reading = ref(false);
  const acknowledged = ref(false);
  const stale = ref(false);
  const notice = ref<'' | 'changed' | 'hostChanged' | 'unconfirmed' | 'resultRecorded'>('');
  const error = ref('');
  let token = 0;
  let readEpoch = 0;
  let disposed = false;
  let versions: ReadonlyMap<string, string> = new Map();
  let observedReviewed = new Set<string>();
  const sourcePrecisionSafe = computed(() => sourceNumbersDisplayExactly(review.value?.source));
  const adoptEligible = computed(() => sourcePrecisionSafe.value && !!review.value?.source.record && review.value.source.provenance?.authority.kind === 'delegated' &&
    !!journal.state.value && sameHost(review.value, journal.state.value) && !journal.state.value.pending &&
    !!approvals.state.value && sameHost(review.value, approvals.state.value) && !approvals.state.value.pending &&
    !!definitions.state.value && sameHost(review.value, definitions.state.value) && !definitions.state.value.pending &&
    !definitions.unavailable.value && !definitions.busy.value && !gate.busy.value &&
    !journal.unavailable.value && !journal.busy.value && !approvals.unavailable.value && !approvals.busy.value && !reading.value && !stale.value);
  const canAdopt = computed(() => adoptEligible.value && acknowledged.value);

  const stopJournalWatch = watch([approvals.state, journal.state, definitions.state], (next, previous) => {
    if (!review.value) return;
    const transitions = (states: typeof next) => states.map((s) => s ? { pending: s.pending, recent: s.recent } : null);
    if (stableJson(transitions(next)) !== stableJson(transitions(previous))) invalidate('changed');
    observeHost();
  }, { flush: 'sync' });

  const stopGateWatch = watch(gate.owner, (owner) => { if (owner && owner !== 'adoption' && review.value) invalidate('changed'); }, { flush: 'sync' });

  function invalidate(reason: 'changed' | 'hostChanged'): void {
    stale.value = true; acknowledged.value = false; notice.value = reason;
  }
  function select(next: ProposalTarget | null): void {
    if (target.value?.kind === next?.kind && target.value?.id === next?.id) return;
    ++token; ++readEpoch;
    target.value = next ? { kind: next.kind, id: next.id } : null;
    observedReviewed.clear();
    review.value = null; reading.value = false; acknowledged.value = false;
    stale.value = false; notice.value = ''; error.value = '';
  }
  function closeReview(): void {
    ++token; ++readEpoch;
    observedReviewed.clear();
    review.value = null; reading.value = false; acknowledged.value = false;
    stale.value = false; notice.value = ''; error.value = '';
  }
  function observe(next: ReadonlyMap<string, string>): void {
    versions = next;
    if (review.value && projectionChanged(review.value, versions, observedReviewed)) invalidate('changed');
  }
  function observeHost(): void {
    if (review.value && [journal.state.value, approvals.state.value, definitions.state.value].some((host) => host && !sameHost(review.value!, host))) invalidate('hostChanged');
  }
  async function refreshJournals(): Promise<void> { await approvals.refresh(); await journal.refresh(); await definitions.refresh(); }
  async function readSaved(): Promise<void> {
    const selected = target.value;
    if (!selected || disposed || reading.value || journal.busy.value || approvals.busy.value || definitions.busy.value || gate.busy.value) return;
    const capturedToken = token;
    const epoch = ++readEpoch;
    reading.value = true; acknowledged.value = false; error.value = ''; notice.value = ''; review.value = null;
    try {
      const host = await journal.refresh();
      await approvals.refresh();
      await definitions.refresh();
      if (!host || disposed || capturedToken !== token || epoch !== readEpoch) return;
      const response = await callWithMeta(control, 'automation.review', { kind: selected.kind, id: selected.id });
      if (disposed || capturedToken !== token || epoch !== readEpoch) return;
      review.value = { ...parseReview(response.result, response.meta, selected), hostId: host.hostId, hostInstanceId: host.hostInstanceId };
      observedReviewed = new Set();
      stale.value = false;
      if (projectionChanged(review.value, versions, observedReviewed)) invalidate('changed');
      observeHost();
    } catch (failure) {
      if (!disposed && capturedToken === token && epoch === readEpoch) error.value = errorMessage(failure);
    } finally {
      if (!disposed && capturedToken === token && epoch === readEpoch) reading.value = false;
    }
  }
  async function adopt(): Promise<boolean> {
    const captured = review.value;
    if (!canAdopt.value || !captured || disposed) return false;
    const capturedToken = token;
    let submitted = false;
    await gate.run('adoption', () => approvals.exclusive(() => journal.exclusive(async () => {
      const other = await approvals.fetchState();
      const host = await journal.fetchState();
      const definition = await definitions.fetchState();
      if (disposed || token !== capturedToken || review.value !== captured) return;
      if (!sameHost(captured, host) || !sameHost(captured, other) || !sameHost(captured, definition)) { invalidate('hostChanged'); return; }
      if (host.pending || other.pending || definition.pending) return;
      const response = await callWithMeta(control, 'automation.review', { kind: captured.source.kind, id: captured.source.id });
      if (disposed || token !== capturedToken || review.value !== captured) return;
      const fresh = parseReview(response.result, response.meta, captured.source);
      if (fresh.fingerprint !== captured.fingerprint || projectionChanged(captured, versions, observedReviewed)) { invalidate('changed'); return; }
      if (!acknowledged.value || stale.value) return;
      const primary = captured.versions.find((v) => recordKey(v.scope as ProposalTarget['kind'], v.id) === recordKey(captured.source.kind, captured.source.id))!;
      const params: AdoptionSubmission = { kind: captured.source.kind, id: captured.source.id, expectedRevision: primary.revision,
        hostId: captured.hostId, hostInstanceId: captured.hostInstanceId, reviewedVersions: captured.versions.map((v) => ({ ...v })), acknowledgeAuthority: true };
      try {
        const next = await journal.command('automation.adoption.submit', { ...params });
        if (!disposed && token === capturedToken) notice.value = next.pending ? 'unconfirmed' : 'resultRecorded';
        submitted = true;
      } catch (failure) {
        if (!disposed && token === capturedToken) { notice.value = 'unconfirmed'; error.value = errorMessage(failure); }
      } finally {
        if (!disposed && token === capturedToken) { acknowledged.value = false; stale.value = true; }
      }
    })));
    if (!submitted && !disposed && token === capturedToken && journal.error.value) {
      error.value = journal.error.value; stale.value = true; acknowledged.value = false;
    }
    return submitted;
  }
  async function reviewUnknown(commandId: string): Promise<void> {
    const captured = journal.state.value;
    if (!captured?.pending || captured.pending.commandId !== commandId || disposed) return;
    const capturedToken = token;
    await journal.exclusive(async () => {
      const current = await journal.fetchState();
      if (disposed || token !== capturedToken || !sameHost(captured, current) || current.pending?.commandId !== commandId) return;
      try { await journal.command('automation.adoption.review_unknown', { commandId, acknowledgeUnknown: true }); }
      finally { if (!disposed && token === capturedToken) invalidate('changed'); }
    });
  }
  async function resolveOriginal(): Promise<void> {
    const capturedToken = token;
    const pending = journal.state.value?.pending;
    await journal.resolve();
    if (pending && !disposed && token === capturedToken) invalidate('changed');
  }
  function dispose(): void { disposed = true; ++token; ++readEpoch; stopJournalWatch(); stopGateWatch(); review.value = null; }
  return { journal, approvals, definitions, gate, target, review, reading, acknowledged, stale, notice, error, sourcePrecisionSafe, adoptEligible, canAdopt, select, closeReview, observe, observeHost, refreshJournals, readSaved, adopt, resolveOriginal, reviewUnknown, dispose };
}
