<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BackupManifest, BackupSummary, StoredBackup } from '@/entities/cloud-account/api/account-types.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

type Mode = 'cloud' | 'file';

const { t, locale } = useI18n();
const account = useAccountStore();
const mode = ref<Mode>('cloud');
const modes = computed(() => [
  { value: 'cloud' as const, label: t('account.backup.modeCloud'), icon: IconCloud },
  { value: 'file' as const, label: t('account.backup.modeFile'), icon: IconFile },
]);

// Cloud mode state
const name = ref('');
const pendingDelete = ref<StoredBackup | null>(null);
const pendingRestore = ref<StoredBackup | null>(null);
const cloudSummary = ref<BackupSummary | null>(null);

// Local file mode state
const fileInput = ref<HTMLInputElement | null>(null);
const fileManifest = ref<BackupManifest | null>(null);
const fileSummary = ref<BackupSummary | null>(null);
const fileDialogOpen = ref(false);
const downloaded = ref(false);

onMounted(() => { void account.refreshBackups(); });

function formatDate(value: string): string {
  return new Date(value).toLocaleString(locale.value, { dateStyle: 'medium', timeStyle: 'short' });
}

function summaryText(summary: BackupSummary | null): string {
  return t('account.backup.summary', {
    workspace: summary?.workspaceSync ?? 0,
    library: (summary?.library ?? 0) + (summary?.mediaLibrary ?? 0),
    assets: summary?.assets ?? 0,
  });
}

function downloadManifest(value: BackupManifest, label: string): void {
  const blob = new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `${label.replace(/[^a-z0-9_-]+/gi, '-') || 'backup'}.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function downloadLocal(): Promise<void> {
  const manifest = await account.createBackup();
  if (manifest) {
    downloadManifest(manifest, 'tiktools-cloud-backup');
    downloaded.value = true;
  }
}

async function uploadLocal(): Promise<void> {
  const manifest = await account.createBackup();
  if (!manifest) return;
  const label = name.value.trim() || `Backup ${new Date().toISOString().slice(0, 10)}`;
  await account.createNamedBackup(label);
  name.value = '';
  downloaded.value = false;
}

async function onFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file) return;
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
  fileSummary.value = await account.previewBackup(fileManifest.value);
  if (fileSummary.value) fileDialogOpen.value = true;
}

async function applyFileRestore(): Promise<void> {
  if (!fileManifest.value) return;
  if (await account.restoreBackup(fileManifest.value)) {
    fileDialogOpen.value = false;
  }
}

async function createCloud(): Promise<void> {
  const label = name.value.trim();
  if (!label) return;
  if (await account.createNamedBackup(label)) name.value = '';
}

async function downloadCloud(backup: StoredBackup): Promise<void> {
  const manifest = await account.getStoredBackup(backup.id);
  if (manifest) downloadManifest(manifest as BackupManifest, backup.name);
}

async function previewCloud(backup: StoredBackup): Promise<void> {
  cloudSummary.value = await account.restoreStoredBackup(backup.id, true);
  if (cloudSummary.value) pendingRestore.value = backup;
}

async function applyCloud(): Promise<void> {
  if (!pendingRestore.value) return;
  if (await account.restoreStoredBackup(pendingRestore.value.id, false)) {
    pendingRestore.value = null;
    cloudSummary.value = null;
  }
}

async function removeCloud(): Promise<void> {
  if (!pendingDelete.value) return;
  if (await account.deleteStoredBackup(pendingDelete.value.id)) pendingDelete.value = null;
}
</script>

<template>
  <BaseGroupBox
    :title="t('account.backup.sectionTitle')"
    :description="t('account.backup.sectionDescription')"
  >
    <div class="backup-section">
      <BaseSegmented
        v-model="mode"
        :options="modes"
        :aria-label="t('account.backup.modeLabel')"
        class="backup-section__modes"
      />

      <template v-if="mode === 'cloud'">
        <p class="backup-section__hint">
          {{ t('account.backup.cloudHint') }}
        </p>
        <div class="backup-section__create">
          <input
            v-model="name"
            class="backup-section__name"
            type="text"
            maxlength="120"
            :placeholder="t('account.backup.namePlaceholder')"
            @keyup.enter="createCloud"
          >
          <BaseButton
            variant="primary"
            :disabled="!name.trim() || account.backupWorking"
            @click="createCloud"
          >
            <template #icon>
              <IconRefresh
                v-if="account.backupWorking"
                class="backup-section__spin"
              />
            </template>
            {{ t('account.backup.saveCloud') }}
          </BaseButton>
        </div>

        <p
          v-if="account.backupError"
          class="backup-section__error"
          role="alert"
        >
          {{ account.backupError }}
        </p>
        <p
          v-if="account.backupsLoading"
          class="backup-section__hint"
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
          class="backup-section__items"
        >
          <li
            v-for="backup in account.backups"
            :key="backup.id"
            class="backup-section__row"
          >
            <span class="backup-section__icon">
              <IconCloud :size="16" />
            </span>
            <div class="backup-section__info">
              <strong>{{ backup.name }}</strong>
              <span class="backup-section__meta">
                {{ formatDate(backup.createdAt) }} · {{ backup.workspaceSync }} / {{ backup.library }} / {{ backup.assets }}
              </span>
            </div>
            <div class="backup-section__actions">
              <BaseButton
                size="sm"
                @click="downloadCloud(backup)"
              >
                <template #icon>
                  <IconDownload :size="14" />
                </template>
                {{ t('account.backup.download') }}
              </BaseButton>
              <BaseButton
                size="sm"
                @click="previewCloud(backup)"
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
      </template>

      <template v-else>
        <p class="backup-section__hint">
          {{ t('account.backup.fileHint') }}
        </p>
        <div class="backup-section__create">
          <BaseButton
            :disabled="account.backupWorking"
            @click="downloadLocal"
          >
            <template #icon>
              <IconDownload :size="14" />
            </template>
            {{ t('account.backup.downloadFile') }}
          </BaseButton>
          <BaseButton
            :disabled="account.backupWorking"
            @click="fileInput?.click()"
          >
            <template #icon>
              <IconUpload :size="14" />
            </template>
            {{ t('account.backup.restoreFile') }}
          </BaseButton>
          <input
            ref="fileInput"
            class="backup-section__file"
            type="file"
            accept="application/json,.json"
            @change="onFile"
          >
        </div>
        <div class="backup-section__upload">
          <input
            v-model="name"
            class="backup-section__name"
            type="text"
            maxlength="120"
            :placeholder="t('account.backup.namePlaceholder')"
            @keyup.enter="uploadLocal"
          >
          <BaseButton
            variant="primary"
            :disabled="!name.trim() || account.backupWorking"
            @click="uploadLocal"
          >
            <template #icon>
              <IconCloud :size="14" />
            </template>
            {{ t('account.backup.uploadToCloud') }}
          </BaseButton>
        </div>
        <p class="backup-section__hint">
          {{ t('account.backup.uploadHint') }}
        </p>
        <p
          v-if="account.backupError"
          class="backup-section__error"
          role="alert"
        >
          {{ account.backupError }}
        </p>
        <p
          v-if="downloaded && !account.backupError"
          class="backup-section__done"
          role="status"
        >
          {{ t('account.backup.done') }}
        </p>
      </template>

      <p class="backup-section__note">
        {{ t('account.backup.excludes') }}
      </p>
    </div>
  </BaseGroupBox>

  <BaseDialog
    :open="pendingDelete !== null"
    :title="t('account.backup.deleteTitle')"
    :width="420"
    @update:open="pendingDelete = null"
  >
    <p>
      {{ t('account.backup.deleteConfirm', { name: pendingDelete?.name ?? '' }) }}
    </p>
    <template #footer>
      <BaseButton @click="pendingDelete = null">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        variant="danger"
        :disabled="account.backupWorking"
        @click="removeCloud"
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
    <p>{{ summaryText(cloudSummary) }}</p>
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
        @click="applyCloud"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
    </template>
  </BaseDialog>

  <BaseDialog
    v-model:open="fileDialogOpen"
    :title="t('account.backup.restore')"
    :width="460"
  >
    <p>{{ summaryText(fileSummary) }}</p>
    <p class="backup-section__note">
      {{ t('account.backup.excludes') }}
    </p>
    <template #footer>
      <BaseButton @click="fileDialogOpen = false">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="account.backupWorking"
        @click="applyFileRestore"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.backup-section { display: grid; gap: var(--space-3); }
.backup-section__modes { width: min(100%, 360px); }
.backup-section__modes :deep(.base-segmented) { width: 100%; }
.backup-section__modes :deep(.base-segmented__option) {
  flex: 1;
  min-width: 0;
  justify-content: center;
  gap: var(--space-2);
}
.backup-section__hint { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.backup-section__note { margin: 0; color: var(--color-text-subtle); font-size: var(--font-size-sm); }
.backup-section__create { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.backup-section__upload { display: flex; flex-wrap: wrap; gap: var(--space-2); padding-top: var(--space-2); border-top: var(--border-width) solid var(--color-border-subtle); }
.backup-section__name {
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
.backup-section__file { display: none; }
.backup-section__items { display: grid; gap: var(--space-2); margin: 0; padding: 0; list-style: none; }
.backup-section__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-raised);
}
.backup-section__icon {
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
.backup-section__info { display: grid; flex: 1; min-width: 0; gap: 2px; }
.backup-section__meta { color: var(--color-text-muted); font-size: var(--font-size-xs); font-variant-numeric: tabular-nums; }
.backup-section__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.backup-section__error { margin: 0; color: var(--color-danger); font-size: var(--font-size-sm); }
.backup-section__done { margin: 0; color: var(--color-success); font-size: var(--font-size-sm); }
.backup-section__spin { animation: backup-section-spin 1s linear infinite; }
@keyframes backup-section-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .backup-section__spin { animation: none; } }
</style>
