import { ref, shallowRef } from 'vue';
import { errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { parseApprovalState, type ApprovalState } from './native-contracts.ts';

function createJournal(control: ControlClient) {
  const state = shallowRef<ApprovalState | null>(null);
  const busy = ref(false);
  const unavailable = ref(true);
  const error = ref('');
  async function fetchState(): Promise<ApprovalState> {
    try {
      const next = parseApprovalState(await control.call('automation.approval.get', {}));
      state.value = next; unavailable.value = false;
      return next;
    } catch (failure) { unavailable.value = true; throw failure; }
  }
  async function exclusive<T>(work: () => Promise<T>): Promise<T | undefined> {
    if (busy.value) return undefined;
    busy.value = true; error.value = '';
    try { return await work(); }
    catch (failure) { error.value = errorMessage(failure); return undefined; }
    finally { busy.value = false; }
  }
  async function refresh(): Promise<ApprovalState | undefined> {
    return exclusive(async () => {
      try { return await fetchState(); }
      catch (failure) { unavailable.value = true; throw failure; }
    });
  }
  async function command(method: string, params: Record<string, unknown>): Promise<ApprovalState> {
    try {
      const next = parseApprovalState(await control.call(method, params));
      state.value = next; unavailable.value = false;
      return next;
    } catch (failure) {
      // The native operation may have committed. Query state; never resend.
      error.value = errorMessage(failure);
      try { await fetchState(); } catch { unavailable.value = true; }
      throw failure;
    }
  }
  const resolve = (): Promise<ApprovalState | undefined> => exclusive(() => command('automation.approval.resolve', {}));
  return { state, busy, unavailable, error, fetchState, exclusive, refresh, command, resolve };
}
export type ApprovalJournal = ReturnType<typeof createJournal>;
const journals = new WeakMap<ControlClient, ApprovalJournal>();
export function approvalJournal(control: ControlClient): ApprovalJournal {
  let journal = journals.get(control);
  if (!journal) { journal = createJournal(control); journals.set(control, journal); }
  return journal;
}
