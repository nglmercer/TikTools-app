import { computed, ref, type ComputedRef } from 'vue';

/**
 * Generations for asynchronous work whose answer must still be wanted when it arrives.
 * `begin()` starts a generation and returns its ticket; `invalidate()` makes every earlier
 * ticket stale. Lists, details and operations that deliver secrets each keep their own
 * scope, so closing a detail never discards a list and a reset never lets a late secret in.
 * A stale ticket only means the UI no longer wants the answer: it cancels nothing remote.
 */
export interface RequestTicket {
  /** Whether no newer `begin()` and no `invalidate()` happened since this ticket was issued. */
  isCurrent(): boolean;
}
export interface RequestScope {
  /** Starts a new generation: every earlier ticket becomes stale. */
  begin(): RequestTicket;
  /** A ticket for the generation in force, without starting one; concurrent callers share it. */
  current(): RequestTicket;
  invalidate(): void;
}

export function createRequestScope(): RequestScope {
  let generation = 0;
  return {
    begin() {
      const issued = ++generation;
      return { isCurrent: () => issued === generation };
    },
    current() {
      const issued = generation;
      return { isCurrent: () => issued === generation };
    },
    invalidate() {
      generation += 1;
    },
  };
}

/**
 * Counts overlapping operations so a nested or concurrent one never clears the busy state
 * of the one that is still running. `reset()` starts a new epoch: operations that were
 * running when the view was reset leave without touching the new view's state.
 */
export interface ActivityCounter {
  readonly active: ComputedRef<boolean>;
  /** Marks one operation as running and returns its (idempotent) exit. */
  enter(): () => void;
  reset(): void;
}

export function createActivityCounter(): ActivityCounter {
  const count = ref(0);
  let epoch = 0;
  return {
    active: computed(() => count.value > 0),
    enter() {
      const entered = epoch;
      count.value += 1;
      let left = false;
      return () => {
        if (left) return;
        left = true;
        if (entered === epoch) count.value -= 1;
      };
    },
    reset() {
      epoch += 1;
      count.value = 0;
    },
  };
}
