import { computed, shallowRef } from 'vue';
import { cloneRecord } from '@/entities/automation/lib/automation-records.ts';
import type { AutomationKind, AutomationRecord, AutomationRecordVersion, AutomationRevision, AutomationWriteBase } from '@/entities/automation/model/record-versions.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';

export interface AutomationEditorGateway {
  read(kind: AutomationKind, id: string): Promise<AutomationRecordVersion>;
  save(kind: AutomationKind, record: AutomationRecord, base: AutomationWriteBase): Promise<AutomationRecordVersion>;
  remove(kind: AutomationKind, id: string, revision: AutomationRevision): Promise<void>;
}
export interface AutomationEdit {
  token: number;
  generation: number;
  kind: AutomationKind;
  candidate: AutomationRecord;
  base: AutomationRecord | null;
  revision: AutomationRevision | undefined;
  create: boolean;
  busy: boolean;
  reviewRequired: boolean;
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
  let disposed = false;
  const dirty = computed(() => !!state.value && (state.value.create || state.value.busy || state.value.reviewRequired || !equal(state.value.base, state.value.candidate)));
  const saveBlocked = computed(() => !state.value || state.value.busy || state.value.reviewRequired || state.value.revision === undefined);
  const canAdoptReview = computed(() => !!state.value && !!review.value && !state.value.busy &&
    state.value.token === review.value.token && state.value.generation === review.value.generation);

  function close(): void {
    ++token; ++readEpoch;
    state.value = null; review.value = null; reading.value = false; readError.value = '';
  }
  function open(kind: AutomationKind, record: AutomationRecord, revision: AutomationRevision | undefined, create = false): void {
    if (disposed) return;
    close();
    state.value = { token, generation: 0, kind, candidate: cloneRecord(record), base: create ? null : cloneRecord(record),
      revision, create, busy: false, reviewRequired: false, failure: null, message: '' };
  }
  function change(record: AutomationRecord): void {
    const current = state.value;
    if (!current || disposed || record.id !== current.candidate.id || equal(record, current.candidate)) return;
    state.value = { ...current, candidate: cloneRecord(record), generation: current.generation + 1 };
  }
  function fail(captured: AutomationEdit, failure: unknown): void {
    const current = state.value;
    if (disposed || current?.token !== captured.token) return;
    const code = failure instanceof ControlCallError ? failure.code : '';
    const outcome = ['revision_conflict', 'automation_not_found'].includes(code) ? 'conflict' :
      ['invalid_params', 'capability_unavailable', 'overloaded'].includes(code) ? 'rejected' : 'unconfirmed';
    state.value = { ...current, busy: false, reviewRequired: true, failure: outcome, message: errorMessage(failure) };
  }
  async function save(): Promise<void> {
    const captured = state.value;
    if (disposed || !captured || !dirty.value || saveBlocked.value || captured.revision === undefined) return;
    const submitted = cloneRecord(captured.candidate);
    state.value = { ...captured, busy: true, message: '' };
    try {
      const result = await gateway.save(captured.kind, submitted, { revision: captured.revision, create: captured.create });
      const current = state.value;
      if (disposed || current?.token !== captured.token) return;
      if (!result.record || result.record.id !== submitted.id) throw new ControlCallError('protocol', 'Missing save acknowledgement.');
      state.value = { ...current, base: cloneRecord(result.record), revision: result.revision, create: false,
        candidate: current.generation === captured.generation ? cloneRecord(result.record) : current.candidate,
        busy: false, reviewRequired: false, failure: null, message: '' };
      review.value = null;
    } catch (failure) { fail(captured, failure); }
  }
  async function remove(): Promise<void> {
    const captured = state.value;
    if (disposed || !captured || captured.create || captured.busy || captured.reviewRequired || captured.revision === undefined) return;
    state.value = { ...captured, busy: true, message: '' };
    try {
      await gateway.remove(captured.kind, captured.candidate.id, captured.revision);
      const current = state.value;
      if (disposed || current?.token !== captured.token) return;
      if (current.generation === captured.generation) close();
      else state.value = { ...current, busy: false, reviewRequired: true, failure: 'conflict', message: '' };
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
      create: compared.current.record === null, generation: current.generation + 1, reviewRequired: false, failure: null, message: '' };
    review.value = null;
    return true;
  }
  function dispose(): void { close(); disposed = true; }
  return { state, review, reading, readError, dirty, saveBlocked, canAdoptReview, open, close, change, save, remove, readForReview, adoptReview, dispose };
}
