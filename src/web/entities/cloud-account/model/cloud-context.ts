import { getCurrentScope, onScopeDispose, watch } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useAccountStore } from './account-store.ts';

export interface CloudContext {
  /** Marker of the account, session and connection the caller is working in right now. */
  stamp(): number;
  /** Whether the account, session and connection are still the ones `stamp` was taken in. */
  isCurrent(stamp: number): boolean;
}

/**
 * Tracks what makes Cloud data and secrets belong to a context: the signed-in account
 * (a logout followed by a login of the same account is a new context too), an account
 * action in progress, the host's `cloud.context.invalidated` notice and a gap in the
 * event stream (a missed notice). `onChange` runs synchronously, before any later render,
 * so private data can be dropped in the same tick. Listeners end with the calling scope.
 */
export function useCloudContext(onChange?: () => void): CloudContext {
  const client = useControlClient();
  const account = useAccountStore();
  let generation = 0;
  const changed = (): void => {
    generation += 1;
    onChange?.();
  };
  watch(
    () => `${account.status?.state ?? ''}\n${account.status?.origin ?? ''}\n${account.status?.accountId ?? ''}`,
    changed,
    { flush: 'sync' },
  );
  // Starting an account action (login, logout, restore) ends the context even before its status arrives.
  watch(() => account.working, (working) => { if (working) changed(); }, { flush: 'sync' });
  const offTopic = client.onTopic('cloud.context.invalidated', changed);
  const offGap = client.onGap(changed);
  if (getCurrentScope()) onScopeDispose(() => { offTopic(); offGap(); });
  return { stamp: () => generation, isCurrent: (stamp) => stamp === generation };
}
