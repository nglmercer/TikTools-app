import { computed, ref, shallowRef } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { UpdateReport } from '../api/package-types.ts';
import { installFailure } from '../lib/install-error.ts';

/** One authoritative update report, including invalidations received during checks. */
export function createPackageUpdateReport(client: Pick<ControlClient, 'call'>) {
  const updates = shallowRef<UpdateReport | null>(null);
  const checkingUpdates = ref(false);
  const updatesError = ref('');
  let updatesRequest = 0;
  let updatesNotificationPending = false;
  const updateCount = computed(() => updates.value?.updates.filter((entry) => entry.latest && entry.latestStatus === 'approved' && !entry.pinned).length ?? 0);

  function invalidateFromNotification(): void {
    if (checkingUpdates.value) updatesNotificationPending = true;
    else void loadUpdates();
  }

  function invalidateUpdateApprovals(): void {
    if (updates.value) updates.value = { ...updates.value, updates: updates.value.updates.map((entry) => ({ ...entry, currentStatus:'unknown', latestStatus:'unknown', autoApplicable:false })) };
  }

  async function loadUpdates(): Promise<void> {
    if (checkingUpdates.value) return;
    const request = ++updatesRequest;
    invalidateUpdateApprovals();
    try {
      const result = await client.call<UpdateReport>('plugins.updates.list', {});
      if (request === updatesRequest) updates.value = result;
    } catch {
      if (request === updatesRequest) updates.value = null;
    }
  }

  async function checkUpdates(): Promise<void> {
    if (checkingUpdates.value) return;
    const request = ++updatesRequest;
    checkingUpdates.value = true;
    updatesError.value = '';
    invalidateUpdateApprovals();
    try {
      const result = await client.call<UpdateReport>('plugins.updates.check', {});
      if (request === updatesRequest) updates.value = result;
    } catch (failure) {
      if (request === updatesRequest) updatesError.value = installFailure(failure).message;
    } finally {
      if (request === updatesRequest) {
        checkingUpdates.value = false;
        if (updatesNotificationPending) {
          updatesNotificationPending = false;
          void loadUpdates();
        }
      }
    }
  }

  return { updates, checkingUpdates, updatesError, updateCount, loadUpdates, checkUpdates, invalidateFromNotification };
}
