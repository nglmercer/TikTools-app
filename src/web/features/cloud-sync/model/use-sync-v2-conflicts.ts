import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { validV2Review, type V2ConflictTarget, type V2ConflictReview } from './recovery-contracts.ts';
import type { SyncRecoveryContext } from './recovery-context.ts';
import { useRecoveryLabels } from './use-recovery-labels.ts';

export function useSyncV2Conflicts(client: ControlClient, context: SyncRecoveryContext) {
  const { t } = useI18n();
  const { v2ConflictValueText } = useRecoveryLabels();
  const v2Conflicts = ref<V2ConflictTarget[]>([]);
  const v2ConflictReview = ref<V2ConflictReview | null>(null);
  const v2ConflictBusy = ref(false);
  const v2ConflictError = ref('');
  const v2ConflictResolved = ref(false);
  async function refreshV2Conflicts(): Promise<void> {
    const generation = context.generation();
    if (!context.available()) return;
    try {
      const result = await client.call<unknown>('sync.v2.conflict.list', {});
      if (generation !== context.generation()) return;
      if (!Array.isArray(result) || !result.every((item) => item != null && typeof item === 'object' && !Array.isArray(item) && typeof (item as Record<string, unknown>).collection === 'string' && typeof (item as Record<string, unknown>).key === 'string')) throw new Error(t('sync.recoveryError'));
      v2Conflicts.value = result as unknown as V2ConflictTarget[];
    } catch (failure) {
      if (generation === context.generation()) v2ConflictError.value = errorMessage(failure);
    }
  }

  async function reviewV2Conflict(collection: string, key: string): Promise<void> {
    if (v2ConflictBusy.value) return;
    const generation = context.generation();
    v2ConflictBusy.value = true; v2ConflictError.value = ''; v2ConflictReview.value = null; v2ConflictResolved.value = false;
    try {
      const result = await client.call<unknown>('sync.v2.conflict.review', { collection, key });
      if (generation !== context.generation()) return;
      if (!validV2Review(result, collection, key)) throw new Error(t('sync.recoveryError'));
      v2ConflictReview.value = result;
    } catch (failure) {
      if (generation === context.generation()) v2ConflictError.value = errorMessage(failure);
    } finally {
      if (generation === context.generation()) v2ConflictBusy.value = false;
    }
  }

  async function resolveV2Conflict(choice: 'keepLocal' | 'takeCloud'): Promise<void> {
    if (!v2ConflictReview.value || v2ConflictBusy.value) return;
    const generation = context.generation();
    v2ConflictBusy.value = true; v2ConflictError.value = '';
    try {
      const outcome = await client.call<string>('sync.v2.conflict.resolve', { handle: v2ConflictReview.value.handle, choice });
      if (generation !== context.generation()) return;
      if (outcome !== 'resolved') throw new Error(t('sync.recoveryError'));
      v2ConflictReview.value = null; v2ConflictResolved.value = true; await refreshV2Conflicts();
    } catch (failure) {
      if (generation === context.generation()) {
        v2ConflictReview.value = null;
        v2ConflictError.value = errorMessage(failure);
        await refreshV2Conflicts();
      }
    } finally {
      if (generation === context.generation()) v2ConflictBusy.value = false;
    }
  }


  function reset(): void {
    v2Conflicts.value = []; v2ConflictReview.value = null; v2ConflictBusy.value = false; v2ConflictError.value = ''; v2ConflictResolved.value = false;
  }
  return { v2Conflicts, v2ConflictReview, v2ConflictBusy, v2ConflictError, v2ConflictResolved, refreshV2Conflicts, reviewV2Conflict, resolveV2Conflict, v2ConflictValueText, reset };
}
export type SyncV2Conflicts = ReturnType<typeof useSyncV2Conflicts>;
