import { watch, type Ref } from 'vue';
import type { ControlClient } from './control-client.ts';
import { ownerMutationGate } from './owner-mutation-gate.ts';

interface RecoverableJournal {
  busy: Readonly<Ref<boolean>>;
  refresh(): Promise<unknown>;
}

/** Reconnect recovers native metadata only. Submitted commands and source
 * candidates remain owned by their existing journals and editors. */
export function recoverOwnerJournalsOnReconnect(control: ControlClient, journals: readonly RecoverableJournal[]): () => void {
  const gate = ownerMutationGate(control);
  let pending = false;
  let running = false;
  let disposed = false;
  async function refresh(): Promise<void> {
    if (disposed || running || !pending || gate.busy.value || journals.some((journal) => journal.busy.value)) return;
    pending = false;
    running = true;
    try { await Promise.allSettled(journals.map((journal) => journal.refresh())); }
    finally {
      running = false;
      if (pending) void refresh();
    }
  }
  const offGap = control.onGap(() => { pending = true; void refresh(); });
  // Batched watch lets each journal finish clearing its in-flight read before
  // retrying a metadata read which overlapped the connection gap.
  const stop = watch(() => [gate.busy.value, ...journals.map((journal) => journal.busy.value)], () => { void refresh(); });
  return () => { disposed = true; pending = false; offGap(); stop(); };
}
