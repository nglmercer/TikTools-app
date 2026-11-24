import { computed, shallowRef } from 'vue';
import { manualReview } from '../../review-shared/model/resolution-preference.ts';
import { cloneRecord } from '@/entities/automation/lib/automation-records.ts';
import type { AutomationHost, AutomationKind, AutomationRecord, AutomationRecordVersion, AutomationRevision, AutomationWriteBase } from '@/entities/automation/model/record-versions.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';

export interface AutomationEditorGateway {
  read(kind: AutomationKind, id: string): Promise<AutomationRecordVersion>;
  save(kind: AutomationKind, record: AutomationRecord, base: AutomationWriteBase): Promise<AutomationRecordVersion>;
  remove(kind: AutomationKind, id: string, revision: AutomationRevision, host?: AutomationHost | null): Promise<void>;
}
export interface AutomationEdit {
  token: number;
  generation: number;
  kind: AutomationKind;
  candidate: AutomationRecord;
  base: AutomationRecord | null;
  revision: AutomationRevision | undefined;
  host?: AutomationHost | null;
  create: boolean;
  busy: boolean;
  reviewRequired: boolean;
  /** True when the conflict/outcome was rebased automatically (default mode). */
  autoResolved: boolean;
  failure: 'conflict' | 'unconfirmed' | 'rejected' | null;
  message: string;
}
export interface AutomationReview {
  token: number;
  generation: number;
  current: AutomationRecordVersion;
}
const equal = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

/** Page-owned draft. Host refreshes never replace its body or advance its base. */
export function useAutomationEditor(gateway: AutomationEditorGateway) {
  const state = shallowRef<AutomationEdit | null>(null);
  const review = shallowRef<AutomationReview | null>(null);
  const reading = shallowRef(false);
  const readError = shallowRef('');
  let token = 0;
  let readEpoch = 0;
  let resolving = false;
  let disposed = false;
  // A new record is not dirty until its draft actually differs from the
  // template it was opened with; create alone must not trigger the discard
  // guard or the "unsaved changes" label.
  const dirty = computed(() => !!state.value && (state.value.busy || state.value.reviewRequired || !equal(state.value.base, state.value.candidate)));
  const saveBlocked = computed(() => !state.value || state.value.busy || state.value.reviewRequired || state.value.revision === undefined);
  const canAdoptReview = computed(() => !!state.value && !!review.value && !state.value.busy &&
    state.value.token === review.value.token && state.value.generation === review.value.generation);

  function close(): void {
    ++token; ++readEpoch;
    state.value = null; review.value = null; reading.value = false; readError.value = '';
  }
  function open(kind: AutomationKind, record: AutomationRecord, revision: AutomationRevision | undefined, create = false, host?: AutomationHost | null): void {
    if (disposed) return;
    close();
    state.value = { token, generation: 0, kind, candidate: cloneRecord(record), base: cloneRecord(record),
      revision, ...(host !== undefined ? { host: host && { ...host } } : {}), create, busy: false, reviewRequired: false, autoResolved: false, failure: null, message: '' };
  }
  function change(record: AutomationRecord): void {
    const current = state.value;
    if (!current || disposed || record.id !== current.candidate.id || equal(record, current.candidate)) return;
    // A fresh local edit supersedes the "record changed" notice: the user has
    // seen it and is now working on the current version.
    state.value = { ...current, candidate: cloneRecord(record), generation: current.generation + 1, autoResolved: false };
  }
  function fail(captured: AutomationEdit, failure: unknown): void {
    const current = state.value;
    if (disposed || current?.token !== captured.token) return;
    const code = failure instanceof ControlCallError ? failure.code : '';
    const outcome = ['revision_conflict', 'automation_not_found', 'not_found'].includes(code) ? 'conflict' :
      ['invalid_params', 'capability_unavailable', 'overloaded', 'unsafe_source', 'command_pending'].includes(code) ? 'rejected' : 'unconfirmed';
    state.value = { ...current, busy: false, reviewRequired: manualReview.value, autoResolved: !manualReview.value, failure: outcome, message: errorMessage(failure) };
    if (!manualReview.value) void autoResolve();
  }
  async function save(): Promise<void> {
    const captured = state.value;
    // A pristine new record still has to be saveable as created.
    if (disposed || !captured || (!dirty.value && !captured.create) || saveBlocked.value || captured.revision === undefined) return;
    const submitted = cloneRecord(captured.candidate);
    state.value = { ...captured, busy: true, message: '' };
    try {
      const result = await gateway.save(captured.kind, submitted, { revision: captured.revision, create: captured.create,
        ...(captured.host !== undefined ? { host: captured.host } : {}) });
      const current = state.value;
      if (disposed || current?.token !== captured.token) return;
      if (!result.record || result.record.id !== submitted.id) throw new ControlCallError('protocol', 'Missing save acknowledgement.');
      state.value = { ...current, base: cloneRecord(result.record), revision: result.revision,
        ...(result.host !== undefined ? { host: result.host && { ...result.host } } : {}), create: false,
        candidate: current.generation === captured.generation ? cloneRecord(result.record) : current.candidate,
        busy: false, reviewRequired: false, autoResolved: false, failure: null, message: '' };
      review.value = null;
    } catch (failure) { fail(captured, failure); }
  }
  async function remove(): Promise<void> {
    const captured = state.value;
    if (disposed || !captured || captured.create || captured.busy || captured.reviewRequired || captured.revision === undefined) return;
    state.value = { ...captured, busy: true, message: '' };
    try {
      if (captured.host === undefined) await gateway.remove(captured.kind, captured.candidate.id, captured.revision);
      else await gateway.remove(captured.kind, captured.candidate.id, captured.revision, captured.host);
      const current = state.value;
      if (disposed || current?.token !== captured.token) return;
      if (current.generation === captured.generation) close();
      else state.value = { ...current, busy: false, reviewRequired: manualReview.value, autoResolved: !manualReview.value, failure: 'conflict', message: '' };
    } catch (failure) { fail(captured, failure); }
  }
  async function readForReview(): Promise<void> {
    const captured = state.value;
    if (disposed || !captured || captured.busy || reading.value) return;
    const epoch = ++readEpoch;
    reading.value = true; readError.value = ''; review.value = null;
    try {
      const current = await gateway.read(captured.kind, captured.candidate.id);
      if (disposed || epoch !== readEpoch || state.value?.token !== captured.token) return;
      // Compare the candidate as it stands when the read completes. Never replace it.
      review.value = { token: captured.token, generation: state.value.generation, current: cloneRecord(current) };
    } catch (failure) {
      if (!disposed && epoch === readEpoch && state.value?.token === captured.token) readError.value = errorMessage(failure);
    } finally { if (!disposed && epoch === readEpoch) reading.value = false; }
  }
  function adoptReview(): boolean {
    const current = state.value;
    const compared = review.value;
    if (!current || !compared || !canAdoptReview.value) return false;
    state.value = { ...current, base: cloneRecord(compared.current.record), revision: compared.current.revision,
      ...(current.host !== undefined || compared.current.host !== undefined ? { host: compared.current.host ?? null } : {}),
      create: compared.current.record === null, generation: current.generation + 1, reviewRequired: false, failure: null, message: '' };
    review.value = null;
    return true;
  }
  function requireRecoveryReview(): void {
    const current = state.value;
    if (!current || disposed || resolving) return;
    // Idempotent: the host watcher runs with flush:'sync' and re-fires on every
    // state write, so a second call would loop forever in automatic mode.
    if (current.reviewRequired || current.autoResolved || current.failure === 'unconfirmed') return;
    ++readEpoch; review.value = null; reading.value = false;
    state.value = { ...current, reviewRequired: manualReview.value, autoResolved: !manualReview.value, failure: 'unconfirmed' };
    if (!manualReview.value) void autoResolve();
  }

  /** Default resolution: re-read the saved record and rebase the draft on it.
   * The original action is never replayed — a timeout is not proof of success. */
  async function autoResolve(): Promise<void> {
    const captured = state.value;
    if (!captured || captured.reviewRequired || captured.busy || disposed || resolving) return;
    resolving = true;
    const token = captured.token;
    try {
      await readForReview();
      if (disposed || state.value?.token !== token) return;
      if (adoptReview()) {
        const current = state.value;
        if (current) state.value = { ...current, autoResolved: true, failure: null, message: '' };
      }
    } finally {
      resolving = false;
    }
  }

  function dispose(): void { close(); disposed = true; }
  return { state, review, reading, readError, dirty, saveBlocked, canAdoptReview, open, close, change, save, remove, readForReview, adoptReview, requireRecoveryReview, dispose };
}
