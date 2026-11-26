<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import type { ChangeReviewItem } from '../../review-shared/model/change-review.ts';
import ChangeReviewWorkspace from '../../review-shared/ui/ChangeReviewWorkspace.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';

const props = defineProps<{ edit: AutomationEdit | null; review: AutomationReview | null; busy: boolean; error: string; canAdopt: boolean }>();
const open = defineModel<boolean>('open', { default: false });
defineEmits<{ adopt: []; refresh: [] }>();
const { t } = useI18n();

/** Both diffs against the starting version as change cards: what the draft
 * changes and what the host changed meanwhile. The semantic field rows are
 * derived from the record JSON, so no per-field mapping lives here. */
const items = computed<ChangeReviewItem[]>(() => {
  const edit = props.edit;
  const review = props.review;
  if (!edit || !review) return [];
  const current = review.current.record;
  return [
    {
      id: 'draft-diff',
      title: t('jsonView.draftChanges'),
      subtitle: edit.revision ?? t('automations.conflict.memory'),
      before: edit.base ?? undefined,
      after: edit.candidate,
    },
    {
      id: 'host-diff',
      title: t('jsonView.hostChanges'),
      subtitle: review.current.revision ?? t('automations.conflict.memory'),
      before: edit.base ?? undefined,
      after: current ?? undefined,
    },
  ];
});
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="automation-review-dialog"
    :title="t('automations.conflict.compareTitle')"
    :width="900"
  >
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
    <p v-if="review && !review.current.record">
      {{ t('automations.conflict.recreateLead') }}
    </p>
    <ChangeReviewWorkspace
      v-if="edit && review"
      :items="items"
      data-test="automation-review-table"
    />
    <template #footer>
      <BaseIconButton
        :label="t('automations.conflict.readCurrent')"
        variant="default"
        :disabled="busy"
        data-test="automation-review-refresh"
        @click="$emit('refresh')"
      >
        <IconRefresh />
      </BaseIconButton>
      <BaseIconButton
        :label="t(review && !review.current.record ? 'automations.conflict.prepareRecreate' : 'automations.conflict.useBase')"
        variant="primary"
        :disabled="!canAdopt || busy"
        data-test="automation-review-adopt"
        @click="$emit('adopt')"
      >
        <IconCheck />
      </BaseIconButton>
      <BaseButton @click="open = false">
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
@media (width < 800px) {
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
