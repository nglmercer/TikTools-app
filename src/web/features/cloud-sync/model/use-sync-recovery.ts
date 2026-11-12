import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { errorMessage, type ControlClient } from '@/shared/api/control-client.ts';
import { validRecoveryReview, type RecoveryReview } from './recovery-contracts.ts';
import type { SyncRecoveryContext } from './recovery-context.ts';
import { useRecoveryLabels } from './use-recovery-labels.ts';

export function useSyncRecovery(client: ControlClient, sync: { refresh(): Promise<void> }, context: SyncRecoveryContext) {
  const { t } = useI18n();
  const { recoveryValueText } = useRecoveryLabels();
  const uncertain = ref<string[]>([]);
  const review = ref<RecoveryReview | null>(null);
  const recoveryBusy = ref(false);
  const recoveryError = ref('');
  const newerNeedsReview = ref(false);
  async function refreshUncertain(): Promise<void> {
    const generation = context.generation();
    if (!context.available()) return;
    try {
      const result = await client.call<unknown>('sync.uncertain.list', {});
      if (!Array.isArray(result) || !result.every((key) => typeof key === 'string' && key.length > 0)) throw new Error(t('sync.recoveryError'));
      if (generation === context.generation()) uncertain.value = result;
    } catch (failure) {
      if (generation === context.generation()) recoveryError.value = errorMessage(failure);
    }
  }
  async function reviewUncertain(key: string): Promise<void> {
    if (recoveryBusy.value) return;
    const generation = context.generation();
    recoveryBusy.value = true; recoveryError.value = ''; review.value = null; newerNeedsReview.value = false;
    try {
      const result = await client.call<unknown>('sync.uncertain.review', { key });
      if (!validRecoveryReview(result, key)) throw new Error(t('sync.recoveryError'));
      if (generation === context.generation()) review.value = result;
    } catch (failure) {
      if (generation === context.generation()) recoveryError.value = errorMessage(failure);
    } finally { if (generation === context.generation()) recoveryBusy.value = false; }
  }
  async function resolveUncertain(choice: 'keepLocal' | 'takeCloud' | 'takeCloudIncludingNewer'): Promise<void> {
    if (!review.value || recoveryBusy.value) return;
    const generation = context.generation();
    recoveryBusy.value = true; recoveryError.value = '';
    try {
      const outcome = await client.call<string>('sync.uncertain.resolve', { handle: review.value.handle, choice });
      if (generation !== context.generation()) return;
      if (outcome === 'newerEditNeedsReview') newerNeedsReview.value = true;
      else if (outcome === 'resolved') { review.value = null; newerNeedsReview.value = false; await refreshUncertain(); await sync.refresh(); }
      else throw new Error(t('sync.recoveryError'));
    } catch (failure) {
      if (generation === context.generation()) {
        review.value = null;
        newerNeedsReview.value = false;
        recoveryError.value = errorMessage(failure);
        await refreshUncertain();
      }
    } finally { if (generation === context.generation()) recoveryBusy.value = false; }
  }


  function reset(): void {
    uncertain.value = []; review.value = null; recoveryError.value = ''; recoveryBusy.value = false; newerNeedsReview.value = false;
  }
  return { uncertain, review, recoveryBusy, recoveryError, newerNeedsReview, refreshUncertain, reviewUncertain, resolveUncertain, recoveryValueText, reset };
}
export type SyncRecovery = ReturnType<typeof useSyncRecovery>;
