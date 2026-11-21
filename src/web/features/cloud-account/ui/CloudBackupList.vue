<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { StoredBackup } from '@/entities/cloud-account/api/account-types.ts';
import { formatBytes } from '@/shared/lib/format-bytes.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconArchive from '@/shared/ui/icon/icons/IconArchive.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';

/** Backups stored in the account, newest first, as a table with one set of actions per row. */
defineProps<{
  backups: StoredBackup[];
  loading: boolean;
  working: boolean;
}>();

const emit = defineEmits<{
  download: [backup: StoredBackup];
  restore: [backup: StoredBackup];
  remove: [backup: StoredBackup];
}>();

const { t, locale } = useI18n();

const columns = computed<TableColumn[]>(() => [
  { key: 'name', label: t('account.backup.columns.name') },
  { key: 'createdAt', label: t('account.backup.columns.date'), width: '170px' },
  { key: 'sizeBytes', label: t('account.backup.columns.size'), width: '90px', align: 'end' },
  { key: 'contents', label: t('account.backup.columns.contents') },
  { key: 'actions', label: '', width: '1%' },
]);

function formatDate(value: string): string {
  return new Date(value).toLocaleString(locale.value, { dateStyle: 'medium', timeStyle: 'short' });
}

function contents(backup: StoredBackup): string {
  return t('account.backup.contents', {
    workspace: backup.workspaceSync,
    library: backup.library + backup.mediaLibrary,
    assets: backup.assets,
  });
}
</script>

<template>
  <p
    v-if="loading && !backups.length"
    class="backup-list__empty"
    role="status"
  >
    <IconRefresh :size="16" />
    {{ t('account.backup.loading') }}
  </p>
  <p
    v-else-if="!backups.length"
    class="backup-list__empty"
    role="status"
  >
    <IconArchive :size="18" />
    {{ t('account.backup.emptyShort') }}
  </p>
  <ul
    v-else
    class="backup-list__cards"
  >
    <li
      v-for="backup in backups"
      :key="backup.id"
      class="backup-list__card"
    >
      <div class="backup-list__card-head">
        <strong class="backup-list__name">{{ backup.name }}</strong>
        <span class="backup-list__muted">{{ formatBytes(backup.sizeBytes, locale) }}</span>
      </div>
      <span class="backup-list__muted">{{ formatDate(backup.createdAt) }} · {{ contents(backup) }}</span>
      <div class="backup-list__card-actions">
        <BaseButton
          variant="primary"
          :disabled="working"
          @click="emit('restore', backup)"
        >
          <template #icon>
            <IconHistory :size="14" />
          </template>
          {{ t('account.backup.restore') }}
        </BaseButton>
        <BaseButton
          :disabled="working"
          @click="emit('download', backup)"
        >
          <template #icon>
            <IconDownload :size="14" />
          </template>
          {{ t('account.backup.download') }}
        </BaseButton>
        <BaseButton
          variant="danger"
          :disabled="working"
          :aria-label="t('account.backup.delete')"
          @click="emit('remove', backup)"
        >
          <template #icon>
            <IconTrash :size="14" />
          </template>
        </BaseButton>
      </div>
    </li>
  </ul>
  <BaseTable
    v-if="backups.length"
    class="backup-list__table"
    :columns="columns"
    :rows="backups"
    :row-key="(backup: StoredBackup) => backup.id"
  >
    <template #cell-name="{ row }">
      <span class="backup-list__name">{{ row.name }}</span>
    </template>
    <template #cell-createdAt="{ row }">
      <span class="backup-list__muted">{{ formatDate(row.createdAt) }}</span>
    </template>
    <template #cell-sizeBytes="{ row }">
      <span class="backup-list__muted">{{ formatBytes(row.sizeBytes, locale) }}</span>
    </template>
    <template #cell-contents="{ row }">
      <span class="backup-list__muted">{{ contents(row) }}</span>
    </template>
    <template #cell-actions="{ row }">
      <span class="backup-list__actions">
        <BaseButton
          size="sm"
          :disabled="working"
          @click="emit('download', row)"
        >
          <template #icon>
            <IconDownload :size="14" />
          </template>
          {{ t('account.backup.download') }}
        </BaseButton>
        <BaseButton
          size="sm"
          :disabled="working"
          @click="emit('restore', row)"
        >
          <template #icon>
            <IconHistory :size="14" />
          </template>
          {{ t('account.backup.restore') }}
        </BaseButton>
        <BaseButton
          size="sm"
          variant="danger"
          :disabled="working"
          @click="emit('remove', row)"
        >
          <template #icon>
            <IconTrash :size="14" />
          </template>
          {{ t('account.backup.delete') }}
        </BaseButton>
      </span>
    </template>
  </BaseTable>
</template>

<style scoped>
.backup-list__name { display: block; min-width: 0; overflow: hidden; font-weight: var(--font-weight-semibold); text-overflow: ellipsis; white-space: nowrap; }
.backup-list__muted { color: var(--color-text-muted); font-size: var(--font-size-sm); font-variant-numeric: tabular-nums; }
.backup-list__actions { display: inline-flex; gap: var(--space-2); }
.backup-list__empty {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  min-height: 56px;
  gap: var(--space-4);
  margin: 0 var(--space-4) var(--space-4);
  padding: var(--space-4);
  border: var(--border-width) dashed var(--color-border);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}
.backup-list__cards { display: none; gap: var(--space-3); margin: 0; padding: 0 var(--space-4) var(--space-4); list-style: none; }
.backup-list__card { display: grid; gap: var(--space-3); padding: var(--space-4); border: var(--border-width) solid var(--color-border-subtle); background: var(--color-bg-raised); }
.backup-list__card-head { display: flex; align-items: baseline; gap: var(--space-3); }
.backup-list__card-head .backup-list__name { flex: 1; }
.backup-list__card-actions { display: grid; grid-template-columns: 1fr 1fr auto; gap: var(--space-2); }
.backup-list__card-actions :deep(.base-button) { min-height: 44px; justify-content: center; }
/* Width of the panel, not the window: narrow columns get cards, wide ones the table. */
@container (width < 640px) {
  .backup-list__table { display: none; }
  .backup-list__cards { display: grid; }
}
</style>
