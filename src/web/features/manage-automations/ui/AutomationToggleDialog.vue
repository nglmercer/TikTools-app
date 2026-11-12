<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { useAutomationToggleReview } from '../model/use-automation-toggle-review.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
defineProps<{ toggle: ReturnType<typeof useAutomationToggleReview> }>();
const emit = defineEmits<{ 'update:open': [open: boolean] }>();
const { t } = useAutomationText();
</script>

<template>
  <BaseConfirmDialog
    :open="toggle.open.value"
    :title="t('automations.conflict.toggleTitle')"
    :message="t(toggle.target.value?.enabled ? 'automations.conflict.enableMessage' : 'automations.conflict.disableMessage', { name: toggle.target.value?.name ?? '' })"
    :confirm-label="t('automations.conflict.confirmToggle')"
    :busy="toggle.confirmBlocked.value"
    @update:open="emit('update:open', $event)"
    @confirm="toggle.confirm"
  >
    <p class="automation-toggle-review__target">
      {{ t('automations.conflict.recordVersion', { id: toggle.target.value?.id ?? '', revision: toggle.target.value?.revision ?? t('automations.conflict.memory') }) }}
    </p>
    <p
      v-if="toggle.error.value"
      role="alert"
    >
      {{ toggle.error.value }}
    </p>
    <BaseButton
      v-if="toggle.needsReview.value"
      :disabled="toggle.busy.value"
      @click="toggle.review"
    >
      {{ t('automations.conflict.readCurrent') }}
    </BaseButton>
  </BaseConfirmDialog>
  <p
    v-if="toggle.error.value && !toggle.open.value"
    class="automation-toggle-review__error"
    role="alert"
  >
    {{ toggle.error.value }}
  </p>
</template>

<style scoped>
.automation-toggle-review__target { overflow-wrap: anywhere; }
.automation-toggle-review__error {
  flex: none;
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}
</style>
