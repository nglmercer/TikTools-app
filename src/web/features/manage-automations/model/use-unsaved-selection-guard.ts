import { ref, shallowRef, watch, type Ref } from 'vue';

/**
 * Runs a selection change at once when nothing is unsaved. Otherwise the change waits as
 * the pending action behind the discard confirmation: confirming runs it, closing the
 * dialog any other way drops it, so a later confirmation can never run an old action.
 */
export function useUnsavedSelectionGuard(dirty: Readonly<Ref<boolean>>): {
  discardOpen: Ref<boolean>;
  guard: (run: () => void) => void;
  confirmDiscard: () => void;
} {
  const pending = shallowRef<(() => void) | null>(null);
  const discardOpen = ref(false);
  watch(discardOpen, (open) => { if (!open) pending.value = null; }, { flush: 'sync' });

  function guard(run: () => void): void {
    if (!dirty.value) {
      run();
      return;
    }
    pending.value = run;
    discardOpen.value = true;
  }

  function confirmDiscard(): void {
    const run = pending.value;
    discardOpen.value = false;
    run?.();
  }

  return { discardOpen, guard, confirmDiscard };
}
