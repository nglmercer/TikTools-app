<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from './BaseDialog.vue';

withDefaults(
  defineProps<{
    title: string;
    message: string;
    confirmLabel: string;
    danger?: boolean;
    busy?: boolean;
  }>(),
  { danger: false, busy: false },
);

const emit = defineEmits<{ confirm: [] }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="title"
    :width="420"
  >
    <p class="base-confirm-dialog__message">
      {{ message }}
    </p>
    <div
      v-if="$slots.default"
      class="base-confirm-dialog__extra"
    >
      <slot />
    </div>
    <template #footer>
      <BaseButton
        :variant="danger ? 'danger' : 'primary'"
        :disabled="busy"
        @click="emit('confirm')"
      >
        {{ confirmLabel }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.base-confirm-dialog__message {
  margin: 0;
  line-height: var(--line-height-normal);
}

.base-confirm-dialog__extra {
  margin-top: var(--space-4);
}
</style>
