<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BackupSummary } from '@/entities/cloud-account/api/account-types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';

/**
 * A backup file the user picked. It can be kept in the account (uploaded as it
 * is) or applied straight away; either way the contents are shown first.
 */
const props = defineProps<{
  open: boolean;
  fileName: string;
  summary: BackupSummary | null;
  working: boolean;
  /** True when the plan's backup count is used up, so uploading is blocked. */
  full: boolean;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  upload: [name: string];
  restore: [];
}>();

const { t } = useI18n();
const name = ref('');

watch(() => [props.open, props.fileName] as const, ([open, file]) => {
  if (open) name.value = file.replace(/\.json$/i, '').slice(0, 120);
});

const label = computed(() => name.value.trim());
const text = computed(() => t('account.backup.summary', {
  workspace: props.summary?.workspaceSync ?? 0,
  library: (props.summary?.library ?? 0) + (props.summary?.mediaLibrary ?? 0),
  assets: props.summary?.assets ?? 0,
}));
</script>

<template>
  <BaseDialog
    :open="open"
    :title="t('account.backup.fileTitle')"
    :width="480"
    @update:open="emit('update:open', $event)"
  >
    <div class="backup-file">
      <p class="backup-file__text">
        {{ text }}
      </p>
      <label class="backup-file__field">
        <span>{{ t('account.backup.fileName') }}</span>
        <input
          v-model="name"
          class="backup-file__input"
          type="text"
          maxlength="120"
          :placeholder="t('account.backup.namePlaceholder')"
        >
      </label>
      <p
        v-if="full"
        class="backup-file__warning"
        role="alert"
      >
        {{ t('account.backup.full') }}
      </p>
      <p class="backup-file__note">
        {{ t('account.backup.excludes') }}
      </p>
    </div>
    <template #footer>
      <BaseButton @click="emit('update:open', false)">
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        :disabled="working"
        @click="emit('restore')"
      >
        {{ t('account.backup.confirm') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="working || !label || full"
        @click="emit('upload', label)"
      >
        {{ t('account.backup.saveFile') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.backup-file { display: grid; gap: var(--space-3); }
.backup-file__text { margin: 0; }
.backup-file__field { display: grid; gap: var(--space-1); color: var(--color-text-muted); font-size: var(--font-size-sm); }
.backup-file__input {
  height: 32px;
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}
.backup-file__warning { margin: 0; color: var(--color-danger); font-size: var(--font-size-sm); }
.backup-file__note { margin: 0; color: var(--color-text-subtle); font-size: var(--font-size-sm); }
</style>
