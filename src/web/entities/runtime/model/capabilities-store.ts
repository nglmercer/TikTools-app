import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createCapabilityLedger, type CapabilityContext, type DiscoverResult } from './capabilities.ts';

/**
 * Runtime method availability for the current host, connection and session.
 * Feature stores ask this before calling optional methods, so an absent Cloud
 * or sync method is reported as unavailable instead of as a failed login.
 */
export const useCapabilityStore = defineStore('runtime-capabilities', () => {
  const client = useControlClient();
  const ledger = createCapabilityLedger();
  // Bumped whenever the snapshot changes so computed readers re-evaluate.
  const revision = ref(0);

  /** Returns `undefined` while no current discovery is known. */
  function supports(method: string): boolean | undefined {
    void revision.value;
    return ledger.supports(method);
  }

  /** Discovers methods for `context`. Resolves `false` when a newer request superseded this one. */
  async function refresh(context: CapabilityContext): Promise<boolean> {
    const ticket = ledger.begin(context);
    const result = await client.call<DiscoverResult>('rpc.discover', {});
    const accepted = ledger.accept(ticket, context, result);
    if (accepted) revision.value += 1;
    return accepted;
  }

  /** Forgets the snapshot, for example when the host or session changes. */
  function invalidate(): void {
    ledger.invalidate();
    revision.value += 1;
  }

  return { supports, refresh, invalidate };
});
