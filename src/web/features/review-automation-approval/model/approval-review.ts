import { computed, ref, shallowRef, watch } from 'vue';
import { callWithMeta, errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { recordKey } from '@/entities/automation/model/record-versions.ts';
import { adoptionJournal } from '@/features/review-automation-proposal/model/adoption-journal.ts';
import { projectionChanged, sameHost, sourceNumbersDisplayExactly, stableJson, type ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';
import { approvalJournal } from './approval-journal.ts';
import { parseApprovalReview, type CapturedApprovalReview } from './contracts.ts';
import type { ApprovalEnablement, ApprovalOperation, ApprovalSubmission } from './native-contracts.ts';

export function useApprovalReview(control: ControlClient) {
  const journal = approvalJournal(control);
  const adoption = adoptionJournal(control);
  const target = shallowRef<ProposalTarget | null>(null);
  const review = shallowRef<CapturedApprovalReview | null>(null);
  const reading = ref(false);
  const acknowledged = ref(false);
  const revokeAcknowledged = ref(false);
  const unknownAcknowledged = ref(false);
  const enablePlan = ref<Record<string, boolean>>({});
  const stale = ref(false);
  const notice = ref<'' | 'changed' | 'hostChanged' | 'unconfirmed' | 'recorded'>('');
  const error = ref('');
  let token = 0;
  let readEpoch = 0;
  let disposed = false;
  let versions: ReadonlyMap<string, string> = new Map();
  let observedReviewed = new Set<string>();
  const sourcePrecisionSafe = computed(() => sourceNumbersDisplayExactly(review.value?.source));
  const transitionReady = computed(() => !!review.value && !!journal.state.value && !!adoption.state.value &&
    sameHost(review.value, journal.state.value) && sameHost(review.value, adoption.state.value) &&
    !journal.state.value.pending && !adoption.state.value.pending && !journal.unavailable.value && !adoption.unavailable.value &&
    !journal.busy.value && !adoption.busy.value && !reading.value && !stale.value);
  const activateEligible = computed(() => transitionReady.value && sourcePrecisionSafe.value && review.value!.eligibility.eligible);
  const revokeEligible = computed(() => transitionReady.value && review.value!.approval.state !== 'none');
  const canActivate = computed(() => activateEligible.value && acknowledged.value);
  const canRevoke = computed(() => revokeEligible.value && revokeAcknowledged.value);

  const stopJournalWatch = watch([journal.state, adoption.state], (next, previous) => {
    if (!review.value) return;
    const transitions = (states: typeof next) => states.map((s) => s ? { pending: s.pending, recent: s.recent } : null);
    if (stableJson(transitions(next)) !== stableJson(transitions(previous))) invalidate('changed');
    observeHost();
  }, { flush: 'sync' });

  function resetConsent(): void { acknowledged.value = false; revokeAcknowledged.value = false; unknownAcknowledged.value = false; }
  function invalidate(reason: 'changed' | 'hostChanged'): void { stale.value = true; resetConsent(); notice.value = reason; }
  function select(next: ProposalTarget | null): void {
    if (target.value?.kind === next?.kind && target.value?.id === next?.id) return;
    ++token; ++readEpoch;
    target.value = next ? { kind: next.kind, id: next.id } : null;
    review.value = null; reading.value = false; enablePlan.value = {}; observedReviewed.clear();
    stale.value = false; notice.value = ''; error.value = ''; resetConsent();
  }
  function closeReview(): void {
    ++token; ++readEpoch; review.value = null; reading.value = false; enablePlan.value = {}; observedReviewed.clear();
    stale.value = false; notice.value = ''; error.value = ''; resetConsent();
  }
  function observe(next: ReadonlyMap<string, string>): void {
    versions = next;
    if (review.value && projectionChanged(review.value, versions, observedReviewed)) invalidate('changed');
  }
  function observeHost(): void {
    if (review.value && [journal.state.value, adoption.state.value].some((host) => host && !sameHost(review.value!, host))) invalidate('hostChanged');
  }
  function includeDependency(id: string, included: boolean): void {
    const dependency = review.value?.source.dependencies.find((d) => d.id === id);
    if (!dependency?.record || dependency.provenance?.authority.kind !== 'delegated') return;
    // Computed properties create an own key even for canonical IDs such as
    // __proto__; assigning next[id] on a plain object would invoke its setter.
    const next = included ? { ...enablePlan.value, [id]: true } : { ...enablePlan.value };
    if (!included) delete next[id];
    enablePlan.value = next; resetConsent();
  }
  function setDependencyEnabled(id: string, enabled: boolean): void {
    if (!Object.hasOwn(enablePlan.value, id)) return;
    enablePlan.value = { ...enablePlan.value, [id]: enabled }; resetConsent();
  }
  function plannedDependencies(captured: CapturedApprovalReview): ApprovalEnablement[] {
    return captured.source.dependencies.flatMap((d) => Object.hasOwn(enablePlan.value, d.id) ? [{ kind: 'action' as const, id: d.id, enabled: enablePlan.value[d.id]! }] : []);
  }
  async function readResponse(selected: ProposalTarget): Promise<CapturedApprovalReview> {
    const target = { kind: selected.kind, id: selected.id };
    const response = await callWithMeta(control, 'automation.approval.review', target);
    return parseApprovalReview(response.result, response.meta, target);
  }
  async function refreshJournals(): Promise<void> { await journal.refresh(); await adoption.refresh(); }
  async function readSaved(): Promise<void> {
    const selected = target.value;
    if (!selected || disposed || reading.value || journal.busy.value || adoption.busy.value) return;
    const capturedToken = token;
    const epoch = ++readEpoch;
    reading.value = true; resetConsent(); error.value = ''; notice.value = '';
    try {
      await refreshJournals();
      if (disposed || token !== capturedToken || epoch !== readEpoch) return;
      const fresh = await readResponse(selected);
      if (disposed || token !== capturedToken || epoch !== readEpoch) return;
      review.value = fresh; enablePlan.value = {}; observedReviewed = new Set(); stale.value = false;
      if (projectionChanged(fresh, versions, observedReviewed)) invalidate('changed');
      observeHost();
    } catch (failure) {
      if (!disposed && token === capturedToken && epoch === readEpoch) { error.value = errorMessage(failure); stale.value = true; }
    } finally { if (!disposed && token === capturedToken && epoch === readEpoch) reading.value = false; }
  }
  async function transition(operation: ApprovalOperation): Promise<boolean> {
    const captured = review.value;
    if (!captured || disposed || !(operation === 'activate' ? canActivate.value : canRevoke.value)) return false;
    const capturedToken = token;
    const capturedPlan = stableJson(enablePlan.value);
    let dispatched = false;
    await journal.exclusive(() => adoption.exclusive(async () => {
      const host = await journal.fetchState();
      const other = await adoption.fetchState();
      if (disposed || token !== capturedToken || review.value !== captured) return;
      if (!sameHost(captured, host) || !sameHost(captured, other)) { invalidate('hostChanged'); return; }
      if (host.pending || other.pending) return;
      const fresh = await readResponse(captured.source);
      if (disposed || token !== capturedToken || review.value !== captured) return;
      if (fresh.fingerprint !== captured.fingerprint || projectionChanged(captured, versions, observedReviewed)) { invalidate('changed'); return; }
      if (stale.value || stableJson(enablePlan.value) !== capturedPlan || !(operation === 'activate' ? acknowledged.value : revokeAcknowledged.value)) return;
      const primary = captured.versions[0]!;
      const common = { kind: captured.source.kind, id: captured.source.id, expectedRevision: primary.revision, hostId: captured.hostId, hostInstanceId: captured.hostInstanceId };
      const input: ApprovalSubmission = operation === 'activate'
        ? { ...common, operation, reviewedVersions: captured.versions.map((v) => ({ ...v })), manifestDigest: captured.manifest!.manifestDigest,
          enablePlan: [{ kind: captured.source.kind, id: captured.source.id, enabled: true }, ...plannedDependencies(captured)], acknowledgeEffects: true }
        : { ...common, operation, acknowledgeRevocation: true };
      try {
        const next = await journal.command('automation.approval.submit', { ...input });
        if (!disposed && token === capturedToken) notice.value = next.pending ? 'unconfirmed' : 'recorded';
        dispatched = true;
      } catch (failure) {
        if (!disposed && token === capturedToken) { notice.value = 'unconfirmed'; error.value = errorMessage(failure); }
      } finally {
        if (!disposed && token === capturedToken) { resetConsent(); stale.value = true; }
      }
    }));
    if (!dispatched && !disposed && token === capturedToken && adoption.error.value) {
      error.value = adoption.error.value; stale.value = true; resetConsent();
    }
    return dispatched;
  }
  async function reviewUnknown(commandId: string): Promise<void> {
    const captured = journal.state.value;
    if (!unknownAcknowledged.value || !captured?.pending || captured.pending.commandId !== commandId || disposed) return;
    const capturedToken = token;
    await journal.exclusive(async () => {
      const current = await journal.fetchState();
      if (disposed || token !== capturedToken || !unknownAcknowledged.value || !sameHost(captured, current) || current.pending?.commandId !== commandId) return;
      try {
        await journal.command('automation.approval.review_unknown', { commandId, acknowledgeUnknown: true });
      } finally {
        if (!disposed && token === capturedToken) invalidate('changed');
      }
    });
  }
  async function resolveOriginal(): Promise<void> {
    const capturedToken = token;
    const pending = journal.state.value?.pending;
    await journal.resolve();
    if (pending && !disposed && token === capturedToken) invalidate('changed');
  }
  function finalDependencyState(id: string): boolean | null {
    if (Object.hasOwn(enablePlan.value, id)) return enablePlan.value[id]!;
    const node = review.value?.source.dependencies.find((d) => d.id === id)?.record;
    return typeof node?.enabled === 'boolean' ? node.enabled : null;
  }
  const reviewVersions = computed(() => new Map(review.value?.versions.map((v) => [recordKey(v.scope as ProposalTarget['kind'], v.id), v.revision]) ?? []));
  function dispose(): void { disposed = true; ++token; ++readEpoch; stopJournalWatch(); review.value = null; resetConsent(); }
  return { journal, adoption, target, review, reading, acknowledged, revokeAcknowledged, unknownAcknowledged, enablePlan, stale, notice, error,
    sourcePrecisionSafe, activateEligible, revokeEligible, canActivate, canRevoke, reviewVersions, select, closeReview, observe, observeHost, includeDependency, setDependencyEnabled,
    readSaved, refreshJournals, activate: () => transition('activate'), revoke: () => transition('revoke'), resolveOriginal, reviewUnknown, finalDependencyState, dispose };
}
