import { ref, shallowRef } from 'vue';
import { errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { parseAdoptionState, type AdoptionState } from './contracts.ts';

/** Shared by every owner review surface using this host connection. Persistence
 * and command IDs belong exclusively to the host's native journal. */
function createJournal(control: ControlClient) {
  const state = shallowRef<AdoptionState | null>(null);
  const busy = ref(false);
  const unavailable = ref(true);
  const error = ref('');

  async function fetchState(): Promise<AdoptionState> {
    const next = parseAdoptionState(await control.call('automation.adoption.get', {}));
    state.value = next;
    unavailable.value = false;
    return next;
  }
  async function exclusive<T>(work: () => Promise<T>): Promise<T | undefined> {
    if (busy.value) return undefined;
    busy.value = true;
    error.value = '';
    try { return await work(); }
    catch (failure) {
      unavailable.value = true;
      error.value = errorMessage(failure);
      return undefined;
    } finally { busy.value = false; }
  }
  async function refresh(): Promise<AdoptionState | undefined> { return exclusive(fetchState); }
  async function command(method: string, params: Record<string, unknown>): Promise<AdoptionState> {
    try {
      const next = parseAdoptionState(await control.call(method, params));
      state.value = next;
      unavailable.value = false;
      return next;
    } catch (failure) {
      // A timeout may follow native persistence/dispatch. Read state, never
      // resend the mutation or invent a replacement command ID.
      error.value = errorMessage(failure);
      try { await fetchState(); }
      catch { unavailable.value = true; }
      throw failure;
    }
  }
  async function resolve(): Promise<AdoptionState | undefined> {
    return exclusive(() => command('automation.adoption.resolve', {}));
  }
  return { state, busy, unavailable, error, refresh, fetchState, exclusive, command, resolve };
}
export type AdoptionJournal = ReturnType<typeof createJournal>;
const journals = new WeakMap<ControlClient, AdoptionJournal>();
export function adoptionJournal(control: ControlClient): AdoptionJournal {
  let journal = journals.get(control);
  if (!journal) { journal = createJournal(control); journals.set(control, journal); }
  return journal;
}
