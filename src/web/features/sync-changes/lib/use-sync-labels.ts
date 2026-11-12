import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SyncAdapter, SyncChange, SyncChangeSide } from '@/entities/cloud-sync/api/sync-adapter.ts';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';

export function useSyncLabels(adapter: Pick<SyncAdapter, 'capabilities'>) {
  const { t } = useI18n();
  const caps = adapter.capabilities;
  const sideIcon: Record<SyncChangeSide, typeof IconUpload> = { local: IconUpload, remote: IconDownload, conflict: IconWarning };
  /** Local changes are "to send" only when the section can send them. */
  const sideKey = computed<Record<SyncChangeSide, string>>(() => ({
    local: caps.push || caps.sync ? 'toSend' : 'localChanges',
    remote: 'toFetch',
    conflict: 'conflicts',
  }));

  const statusLabel = (change: SyncChange): string => t(`syncChanges.status.${change.status}`);
  return { sideIcon, sideKey, statusLabel };
}
