<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BackupSummary, StoredBackup } from '@/entities/cloud-account/api/account-types.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';

const { t, locale } = useI18n();
const account = useAccountStore();
const name = ref('');
const pendingDelete = ref<StoredBackup | null>(null);
const pendingRestore = ref<StoredBackup | null>(null);
const summary = ref<BackupSummary | null>(null);

onMounted(() => { void account.refreshBackups(); });

function formatDate(value: string): string {
  return new Date(value).toLocaleString(locale.value, { dateStyle: 'medium', timeStyle: 'short' });
}

async function create(): Promise<void> {
  const label = name.value.trim();
  if (!label) return;
  const created = await account.createNamedBackup(label);
  if (created) name.value = '';
}

async function download(backup: StoredBackup): Promise<void> {
  const manifest = await account.getStoredBackup(backup.id);
  if (!manifest) return;
  const blob = new Blob([JSON.stringify(manifest, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `${backup.name.replace(/[^a-z0-9_-]+/gi, '-') || 'backup'}.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function preview(backup: StoredBackup): Promise<void> {
  summary.value = await account.restoreStoredBackup(backup.id, true);
  if (summary.value) pendingRestore.value = backup;
}

async function apply(): Promise<void> {
  if (!pendingRestore.value) return;
  const result = await account.restoreStoredBackup(pendingRestore.value.id, false);
  if (result) { pendingRestore.value = null; summary.value = null; }
}

async function remove(): Promise<void> {
  if (!pendingDelete.value) return;
  if (await account.deleteStoredBackup(pendingDelete.value.id)) pendingDelete.value = null;
}
</script>

<template>
  <BaseGroupBox
    :title="t('account.backup.listTitle')"
    :description="t('account.backup.listDescription')"
  >
    <div class="cloud-backup-list">
      <div class="cloud-backup-list__create">
        <input
          v-model="name"
          class="cloud-backup-list__name"
          type="text"
          maxlength="120"
          :placeholder="t('account.backup.namePlaceholder')"
          @keyup.enter="create"
        >
        <BaseButton
          variant="primary"
          :disabled="!name.trim() || account.backupWorking"
          @click="create"
        >
          {{ t('account.backup.createNamed') }}
        </BaseButton>
      </div>

      <p
        v-if="account.backupError"
        class="cloud-backup-list__error"
        role="alert"
      >
        {{ account.backupError }}
      </p>
      <p
        v-if="account.backupsLoading"
        class="cloud-backup-list__hint"
        role="status"
      >
        {{ t('account.backup.loading') }}
      </p>

      <BaseEmptyState
        v-else-if="!account.backups.length"
        :title="t('account.backup.empty')"
        :description="t('account.backup.emptyHint')"
      >
        <template #icon>
          <IconCloud :size="28" />
        </template>
      </BaseEmptyState>

      <ul
        v-else
        class="cloud-backup-list__items"
      >
        <li
          v-for="backup in account.backups"
          :key="backup.id"
          class="cloud-backup-list__row"
        >
          <span class="cloud-backup-list__icon">
            <IconCloud :size="16" />
          </span>
          <div class="cloud-backup-list__info">
            <strong>{{ backup.name }}</strong>
            <span class="cloud-backup-list__meta">
              {{ formatDate(backup.createdAt) }} · {{ backup.workspaceSync }} / {{ backup.library }} / {{ backup.assets }}
              <BaseStatus
                :label="t('account.backup.stored')"
                tone="info"
              />
            </span>
          </div>
          <div class="cloud-backup-list__actions">
            <BaseButton
              size="sm"
              @click="download(backup)"
            >
              <template #icon>
                <IconDownload :size="14" />
              </template>
              {{ t('account.backup.download') }}
            </BaseButton>
            <BaseButton
              size="sm"
              @click="preview(backup)"
            >
              {{ t('account.backup.restore') }}
            </BaseButton>
            <BaseButton
              size="sm"
              variant="danger"
              :disabled="account.backupWorking"
              @click="pendingDelete = backup"
            >
              <template #icon>
                <IconTrash :size="14" />
              </template>
              {{ t('account.backup.delete') }}
            </BaseButton>
          </div>
        </li>
      </ul>
    </div>
  </BaseGroupBox>

  <BaseDialog
    :open="pendingDelete !== null"
    :title="t('account.backup.deleteTitle')"
    :width="420"
    @update:open="pendingDelete = null"
  >
    <p>{{ t('account.backup.deleteConfirm', { name: pendingDelete?.name ?? '' }) }}</p>
    <template #footer>
      <BaseButton @click="pendingDelete = null">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        variant="danger"
        :disabled="account.backupWorking"
        @click="remove"
      >
        {{ t('account.backup.delete') }}
      </BaseButton>
    </template>
  </BaseDialog>

  <BaseDialog
    :open="pendingRestore !== null"
    :title="t('account.backup.restore')"
    :width="460"
    @update:open="pendingRestore = null"
  >
    <p>
      {{ t('account.backup.summary', {
        workspace: summary?.workspaceSync ?? 0,
        library: (summary?.library ?? 0) + (summary?.mediaLibrary ?? 0),
        assets: summary?.assets ?? 0,
      }) }}
    </p>
    <p class="cloud-backup-list__hint">
      {{ t('account.backup.excludes') }}
    </p>
    <template #footer>
      <BaseButton @click="pendingRestore = null">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="account.backupWorking"
        @click="apply"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.cloud-backup-list { display: grid; gap: var(--space-3); }
.cloud-backup-list__create { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.cloud-backup-list__name {
  flex: 1;
  min-width: 160px;
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}
.cloud-backup-list__items { display: grid; gap: var(--space-2); margin: 0; padding: 0; list-style: none; }
.cloud-backup-list__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-raised);
}
.cloud-backup-list__icon {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-accent);
}
.cloud-backup-list__info { display: grid; flex: 1; min-width: 0; gap: 2px; }
.cloud-backup-list__meta { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2); color: var(--color-text-muted); font-size: var(--font-size-xs); font-variant-numeric: tabular-nums; }
.cloud-backup-list__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.cloud-backup-list__hint { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.cloud-backup-list__error { margin: 0; color: var(--color-danger); font-size: var(--font-size-sm); }
</style>
