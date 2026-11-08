<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';

defineProps<{ edit: AutomationEdit | null; review: AutomationReview | null; busy: boolean; error: string; canAdopt: boolean }>();
const open = defineModel<boolean>('open', { default: false });
defineEmits<{ adopt: []; refresh: [] }>();
const { t } = useI18n();
const text = (record: unknown): string => JSON.stringify(record, null, 2);
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="automation-review-dialog"
    :title="t('automations.conflict.compareTitle')"
    :width="1080"
  >
    <p>{{ t('automations.conflict.compareLead') }}</p>
    <p
      v-if="busy"
      role="status"
    >
      {{ t('automations.conflict.reading') }}
    </p>
    <p
      v-if="error"
      role="alert"
    >
      {{ error }}
    </p>
    <div
      v-if="edit && review"
      class="automation-review"
    >
      <section>
        <h3>{{ t('automations.conflict.base') }} · {{ edit.revision ?? t('automations.conflict.memory') }}</h3>
        <pre tabindex="0">{{ edit.base ? text(edit.base) : t('automations.conflict.absent') }}</pre>
      </section>
      <section>
        <h3>{{ t('automations.conflict.candidate') }}</h3>
        <pre tabindex="0">{{ text(edit.candidate) }}</pre>
      </section>
      <section>
        <h3>{{ t('automations.conflict.current') }} · {{ review.current.revision ?? t('automations.conflict.memory') }}</h3>
        <pre tabindex="0">{{ review.current.record ? text(review.current.record) : t('automations.conflict.deleted') }}</pre>
      </section>
    </div>
    <p v-if="review && !review.current.record">
      {{ t('automations.conflict.recreateLead') }}
    </p>
    <p v-else-if="review">
      {{ t('automations.conflict.mergeLead') }}
    </p>
    <template #footer>
      <BaseButton
        :disabled="busy"
        @click="$emit('refresh')"
      >
        {{ t('automations.conflict.readCurrent') }}
      </BaseButton>
      <BaseButton
        :disabled="!canAdopt || busy"
        @click="$emit('adopt')"
      >
        {{ t(review && !review.current.record ? 'automations.conflict.prepareRecreate' : 'automations.conflict.useBase') }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.automation-review {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-4);
}
.automation-review section { min-width: 0; }
.automation-review h3 { font-size: var(--font-size-md); overflow-wrap: anywhere; }
.automation-review pre {
  max-height: 48vh;
  padding: var(--space-3);
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  border: var(--border-width) solid var(--color-border);
}
@media (width < 800px) {
  .automation-review { grid-template-columns: 1fr; }
  .automation-review pre { max-height: 28vh; }
  .automation-review-dialog :deep(.base-dialog__footer) { flex-direction: column; align-items: stretch; }
  .automation-review-dialog :deep(.base-dialog__spacer) { display: none; }
  .automation-review-dialog :deep(.base-dialog__footer .base-button) {
    height: auto;
    min-height: var(--control-height-md);
    padding: var(--space-2) var(--space-3);
    white-space: normal;
  }
}
</style>
