<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import type { BackupManifest, BackupSummary } from '@/entities/cloud-account/api/account-types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';

const { t } = useI18n();
const account = useAccountStore();
const fileInput = ref<HTMLInputElement | null>(null);
const dialogOpen = ref(false);
const manifest = ref<BackupManifest | null>(null);
const summary = ref<BackupSummary | null>(null);
const done = ref(false);

function downloadBackup(value: BackupManifest): void {
  const blob = new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `tiktools-cloud-backup-${new Date().toISOString().slice(0, 10)}.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function createBackup(): Promise<void> {
  const backup = await account.createBackup();
  if (backup) {
    downloadBackup(backup);
    done.value = true;
  }
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
    manifest.value = parsed as BackupManifest;
  } catch {
    account.backupError = t('account.backup.invalid');
    return;
  }
  done.value = false;
  summary.value = await account.previewBackup(manifest.value);
  if (summary.value) dialogOpen.value = true;
}

async function applyRestore(): Promise<void> {
  if (!manifest.value) return;
  const result = await account.restoreBackup(manifest.value);
  if (result) {
    dialogOpen.value = false;
    done.value = true;
  }
}
</script>

<template>
  <BaseGroupBox
    :title="t('account.backup.title')"
    :description="t('account.backup.description')"
  >
    <div class="cloud-backup">
      <div class="cloud-backup__actions">
        <BaseButton
          :disabled="account.backupWorking"
          @click="createBackup"
        >
          <template #icon>
            <IconRefresh
              v-if="account.backupWorking"
              class="cloud-backup__spin"
            />
          </template>
          {{ t('account.backup.create') }}
        </BaseButton>
        <BaseButton
          :disabled="account.backupWorking"
          @click="fileInput?.click()"
        >
          {{ t('account.backup.restore') }}
        </BaseButton>
        <input
          ref="fileInput"
          class="cloud-backup__file"
          type="file"
          accept="application/json,.json"
          @change="onFile"
        >
      </div>
      <p class="cloud-backup__note">
        {{ t('account.backup.excludes') }}
      </p>
      <p
        v-if="account.backupError"
        class="cloud-backup__error"
        role="alert"
      >
        {{ account.backupError }}
      </p>
      <p
        v-if="done && !account.backupError"
        class="cloud-backup__done"
        role="status"
      >
        {{ t('account.backup.done') }}
      </p>
    </div>
  </BaseGroupBox>

  <BaseDialog
    v-model:open="dialogOpen"
    :title="t('account.backup.restore')"
    :width="460"
  >
    <p class="cloud-backup__summary">
      {{ t('account.backup.summary', {
        workspace: summary?.workspaceSync ?? 0,
        library: (summary?.library ?? 0) + (summary?.mediaLibrary ?? 0),
        assets: summary?.assets ?? 0,
      }) }}
    </p>
    <p class="cloud-backup__note">
      {{ t('account.backup.excludes') }}
    </p>
    <template #footer>
      <BaseButton
        variant="primary"
        :disabled="!summary || account.backupWorking"
        @click="applyRestore"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.cloud-backup { display: grid; gap: var(--space-2); }
.cloud-backup__actions { display: flex; flex-wrap: wrap; gap: var(--space-3); }
.cloud-backup__file { display: none; }
.cloud-backup__note { margin: 0; color: var(--color-text-subtle); font-size: var(--font-size-sm); }
.cloud-backup__summary { margin: 0; font-size: var(--font-size-md); }
.cloud-backup__error { margin: 0; color: var(--color-danger); font-size: var(--font-size-sm); }
.cloud-backup__done { margin: 0; color: var(--color-success); font-size: var(--font-size-sm); }
.cloud-backup__spin { animation: cloud-backup-spin 1s linear infinite; }
@keyframes cloud-backup-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .cloud-backup__spin { animation: none; } }
</style>
