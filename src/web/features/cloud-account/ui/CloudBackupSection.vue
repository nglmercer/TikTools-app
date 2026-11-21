<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BackupManifest, BackupSummary, StoredBackup } from '@/entities/cloud-account/api/account-types.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconCloudCheck from '@/shared/ui/icon/icons/IconCloudCheck.vue';
import IconCloudUpload from '@/shared/ui/icon/icons/IconCloudUpload.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconHardDrive from '@/shared/ui/icon/icons/IconHardDrive.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconShield from '@/shared/ui/icon/icons/IconShield.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import { suggestBackupName } from '@/entities/cloud-account/lib/backup-name.ts';
import GuideCatalogButton from '@/features/guided-help/ui/GuideCatalogButton.vue';
import CloudBackupAction from './CloudBackupAction.vue';
import CloudBackupFileDialog from './CloudBackupFileDialog.vue';
import CloudBackupList from './CloudBackupList.vue';

const { t } = useI18n();
const account = useAccountStore();

type Mode = 'cloud' | 'local';
const mode = ref<Mode>('cloud');
const modes = computed(() => [
  { value: 'cloud' as const, label: t('account.backup.tabCloud'), icon: IconCloud, badge: account.backups.length },
  { value: 'local' as const, label: t('account.backup.tabLocal'), icon: IconHardDrive },
]);

// The name is pre-filled so one click saves a backup; the user can still rename it.
const suggestion = () => suggestBackupName(t('account.backup.namePrefix'));
const name = ref(suggestion());
const notice = ref('');
const fileInput = ref<HTMLInputElement | null>(null);

// A backup stored in the account that is about to be restored or deleted.
const pendingDelete = ref<StoredBackup | null>(null);
const pendingRestore = ref<StoredBackup | null>(null);
const restoreSummary = ref<BackupSummary | null>(null);

// A backup file picked from disk.
const fileManifest = ref<BackupManifest | null>(null);
const fileName = ref('');
const fileSummary = ref<BackupSummary | null>(null);
const fileDialogOpen = ref(false);

onMounted(() => {
  void account.refreshBackups();
  if (!account.plan) void account.refreshPlan();
});

const full = computed(() => {
  const plan = account.plan;
  return plan ? plan.usage.backups >= plan.limits.maxBackups : false;
});

function summaryText(summary: BackupSummary | null): string {
  return t('account.backup.summary', {
    workspace: summary?.workspaceSync ?? 0,
    library: (summary?.library ?? 0) + (summary?.mediaLibrary ?? 0),
    assets: summary?.assets ?? 0,
  });
}

function saveAsFile(manifest: unknown, label: string): void {
  const blob = new Blob([JSON.stringify(manifest, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `${label.replace(/[^a-z0-9_-]+/gi, '-') || 'backup'}.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function create(): Promise<void> {
  const label = name.value.trim();
  if (!label || full.value) return;
  notice.value = '';
  if (await account.createNamedBackup(label)) {
    name.value = suggestion();
    notice.value = t('account.backup.created');
  }
}

async function exportCurrent(): Promise<void> {
  notice.value = '';
  const manifest = await account.createBackup();
  if (manifest) {
    saveAsFile(manifest, 'tiktools-cloud-backup');
    notice.value = t('account.backup.done');
  }
}

async function onFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;
  notice.value = '';
  try {
    const parsed: unknown = JSON.parse(await file.text());
    if (!parsed || typeof parsed !== 'object' || (parsed as BackupManifest).schemaVersion !== 1) {
      throw new Error('invalid backup');
    }
    fileManifest.value = parsed as BackupManifest;
  } catch {
    account.backupError = t('account.backup.invalid');
    return;
  }
  fileName.value = file.name;
  fileSummary.value = await account.previewBackup(fileManifest.value);
  if (fileSummary.value) fileDialogOpen.value = true;
}

async function uploadFile(label: string): Promise<void> {
  if (!fileManifest.value) return;
  if (await account.createNamedBackup(label, fileManifest.value)) {
    fileDialogOpen.value = false;
    fileManifest.value = null;
    notice.value = t('account.backup.uploaded');
  }
}

async function restoreFile(): Promise<void> {
  if (!fileManifest.value) return;
  if (await account.restoreBackup(fileManifest.value)) {
    fileDialogOpen.value = false;
    fileManifest.value = null;
    notice.value = t('account.backup.restored');
  }
}

async function download(backup: StoredBackup): Promise<void> {
  const manifest = await account.getStoredBackup(backup.id);
  if (manifest) saveAsFile(manifest, backup.name);
}

async function previewRestore(backup: StoredBackup): Promise<void> {
  restoreSummary.value = await account.restoreStoredBackup(backup.id, true);
  if (restoreSummary.value) pendingRestore.value = backup;
}

async function applyRestore(): Promise<void> {
  if (!pendingRestore.value) return;
  if (await account.restoreStoredBackup(pendingRestore.value.id, false)) {
    pendingRestore.value = null;
    restoreSummary.value = null;
    notice.value = t('account.backup.restored');
  }
}

async function remove(): Promise<void> {
  if (!pendingDelete.value) return;
  if (await account.deleteStoredBackup(pendingDelete.value.id)) pendingDelete.value = null;
}
</script>

<template>
  <BasePanel
    class="backup-section"
    :title="t('account.backup.sectionTitle')"
    header-layout="inline"
    flush
  >
    <template #actions>
      <GuideCatalogButton
        category="account"
        :task-state="() => ({ 'backup.saved': account.backups.length > 0 })"
      />
    </template>

    <BaseTabs
      v-model="mode"
      class="backup-section__tabs"
      data-guide="backup-tools"
      :tabs="modes"
      :label="t('account.backup.modeLabel')"
    >
      <div
        v-if="mode === 'cloud'"
        class="backup-section__pane"
      >
        <form
          class="backup-section__create"
          data-guide="backup-create"
          @submit.prevent="create"
        >
          <BaseTextField
            v-model="name"
            class="backup-section__name"
            maxlength="120"
            :aria-label="t('account.backup.namePlaceholder')"
            :placeholder="t('account.backup.namePlaceholder')"
          />
          <BaseButton
            type="submit"
            variant="primary"
            :disabled="!name.trim() || account.backupWorking || full"
          >
            <template #icon>
              <IconRefresh
                v-if="account.backupWorking"
                class="backup-section__spin"
                :size="14"
              />
              <IconCloudUpload
                v-else
                :size="14"
              />
            </template>
            {{ t('account.backup.saveCloud') }}
          </BaseButton>
        </form>
        <p class="backup-section__hint">
          <IconCloudCheck :size="16" />
          {{ t('account.backup.cloudHint') }}
        </p>
      </div>
      <div
        v-else
        class="backup-section__pane"
      >
        <p class="backup-section__hint">
          <IconHardDrive :size="16" />
          {{ t('account.backup.localHint') }}
        </p>
        <CloudBackupAction
          :icon="IconDownload"
          :title="t('account.backup.exportTitle')"
          :description="t('account.backup.exportDesc')"
        >
          <BaseButton
            :disabled="account.backupWorking"
            @click="exportCurrent"
          >
            <template #icon>
              <IconDownload :size="14" />
            </template>
            {{ t('account.backup.downloadFile') }}
          </BaseButton>
        </CloudBackupAction>
        <CloudBackupAction
          :icon="IconUpload"
          :title="t('account.backup.importTitle')"
          :description="t('account.backup.importDesc')"
        >
          <BaseButton
            :disabled="account.backupWorking"
            @click="fileInput?.click()"
          >
            <template #icon>
              <IconUpload :size="14" />
            </template>
            {{ t('account.backup.uploadFile') }}
          </BaseButton>
        </CloudBackupAction>
        <input
          ref="fileInput"
          class="backup-section__file"
          type="file"
          accept="application/json,.json"
          tabindex="-1"
          @change="onFile"
        >
      </div>
    </BaseTabs>
    <p
      v-if="account.backupError"
      class="backup-section__message backup-section__message--error"
      role="alert"
    >
      {{ account.backupError }}
    </p>
    <p
      v-else-if="full && mode === 'cloud'"
      class="backup-section__message backup-section__message--error"
      role="status"
    >
      {{ t('account.backup.full') }}
    </p>
    <p
      v-else-if="notice"
      class="backup-section__message backup-section__message--done"
      role="status"
    >
      {{ notice }}
    </p>

    <div
      v-if="mode === 'cloud'"
      class="backup-section__list"
      data-guide="backup-list"
    >
      <CloudBackupList
        :backups="account.backups"
        :loading="account.backupsLoading"
        :working="account.backupWorking"
        @download="download"
        @restore="previewRestore"
        @remove="pendingDelete = $event"
      />
    </div>

    <template #footer>
      <p class="backup-section__note">
        <IconShield :size="14" />
        {{ t('account.backup.excludes') }}
      </p>
    </template>
  </BasePanel>

  <CloudBackupFileDialog
    v-model:open="fileDialogOpen"
    :file-name="fileName"
    :summary="fileSummary"
    :working="account.backupWorking"
    :full="full"
    @upload="uploadFile"
    @restore="restoreFile"
  />

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
    <p>{{ summaryText(restoreSummary) }}</p>
    <p class="backup-section__note">
      {{ t('account.backup.excludes') }}
    </p>
    <template #footer>
      <BaseButton @click="pendingRestore = null">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="account.backupWorking"
        @click="applyRestore"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.backup-section__file { display: none; }
.backup-section__tabs :deep(.base-tabs__bar) { padding: 0 var(--space-3); }
.backup-section { container-type: inline-size; flex: 1; }
.backup-section :deep(.base-panel__body) { display: flex; flex-direction: column; }
.backup-section__tabs { flex: none; }
.backup-section__list { display: flex; flex: 1; flex-direction: column; min-height: 0; }
.backup-section__pane { display: grid; gap: var(--space-3); padding: var(--space-4); }
.backup-section__hint { display: flex; align-items: center; gap: var(--space-3); margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.backup-section__create { display: flex; flex-wrap: wrap; gap: var(--space-3); }
.backup-section__name { flex: 1 1 200px; min-width: 0; }
.backup-section__quota { font-variant-numeric: tabular-nums; }
.backup-section__quota--full { color: var(--color-danger-hover); }
.backup-section__message { margin: 0; padding: var(--space-3) var(--space-4); font-size: var(--font-size-sm); }
.backup-section__message--error { color: var(--color-danger); }
.backup-section__message--done { color: var(--color-success); }
.backup-section__note { display: flex; align-items: center; gap: var(--space-3); margin: 0; padding: var(--space-2) var(--space-4); color: var(--color-text-subtle); font-size: var(--font-size-sm); }
.backup-section__spin { animation: backup-section-spin 1s linear infinite; }
@keyframes backup-section-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .backup-section__spin { animation: none; } }
@container (width < 480px) {
  .backup-section__tabs :deep(.base-tabs__list) { display: grid; flex: 1; grid-template-columns: 1fr 1fr; }
  .backup-section__tabs :deep(.base-tabs__tab) { justify-content: center; min-height: 44px; }
  .backup-section__create { flex-direction: column; }
  .backup-section__name { flex: none; width: 100%; }
  .backup-section__create :deep(.base-button), .backup-section__name :deep(input) { min-height: 44px; }
  .backup-section__create :deep(.base-button) { justify-content: center; }
  .backup-section__pane :deep(.backup-action__control), .backup-section__pane :deep(.backup-action__control .base-button) { width: 100%; }
  .backup-section__pane :deep(.backup-action__control .base-button) { min-height: 44px; justify-content: center; }
}
</style>
