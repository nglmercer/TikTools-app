import { ref } from 'vue';
import { lifecycleFailure, type LifecycleFailureKind } from '@/shared/api/cloud-lifecycle.ts';

/** The last failed action on one Cloud item, so a list can explain it next to that item. */
export interface ItemFailure<Action extends string = string> {
  readonly id: string;
  readonly action: Action;
  readonly kind: LifecycleFailureKind;
  /** The host's own text, shown only when the kind has no wording of its own. */
  readonly message: string;
}

export function useItemFailure<Action extends string>() {
  const failure = ref<ItemFailure<Action> | null>(null);
  return {
    failure,
    set(id: string, action: Action, cause: unknown): LifecycleFailureKind {
      const kind = lifecycleFailure(cause);
      failure.value = { id, action, kind, message: cause instanceof Error ? cause.message : String(cause) };
      return kind;
    },
    clear(): void {
      failure.value = null;
    },
  };
}
