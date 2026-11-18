<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import type { ReviewRow } from '../../review-shared/model/review-rows.ts';
import ReviewComparisonTable from '../../review-shared/ui/ReviewComparisonTable.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';

const props = defineProps<{ edit: AutomationEdit | null; review: AutomationReview | null; busy: boolean; error: string; canAdopt: boolean }>();
const open = defineModel<boolean>('open', { default: false });
defineEmits<{ adopt: []; refresh: [] }>();
const { t } = useI18n();

/** One view-model for the three versions and their two diffs. Both diffs used
 * to re-run diffJson on the same base; the table now renders them from here. */
const rows = computed<ReviewRow[]>(() => {
  const edit = props.edit;
  const review = props.review;
  if (!edit || !review) return [];
  const current = review.current.record;
  const result: ReviewRow[] = [
    {
      id: 'base',
      name: t('automations.conflict.base'),
      kind: 'version',
      change: 'version',
      revision: edit.revision ?? t('automations.conflict.memory'),
      note: edit.base ? undefined : t('automations.conflict.absent'),
      after: edit.base ?? undefined,
    },
    {
      id: 'candidate',
      name: t('automations.conflict.candidate'),
      kind: 'version',
      change: 'version',
      after: edit.candidate,
    },
    {
      id: 'current',
      name: t('automations.conflict.current'),
      kind: 'version',
      change: current ? 'version' : 'remove',
      revision: review.current.revision ?? t('automations.conflict.memory'),
      note: current ? undefined : t('automations.conflict.deleted'),
      after: current ?? undefined,
    },
    {
      id: 'draft-diff',
      name: t('jsonView.draftChanges'),
      kind: 'diff',
      change: 'diff',
      before: edit.base ?? undefined,
      after: edit.candidate,
    },
    {
      id: 'host-diff',
      name: t('jsonView.hostChanges'),
      kind: 'diff',
      change: 'diff',
      before: edit.base ?? undefined,
      after: current ?? undefined,
    },
  ];
  return result;
});
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
    <ReviewComparisonTable
      v-if="edit && review"
      :rows="rows"
      data-test="automation-review-table"
      docs-path="developer/automations.html"
      docs-anchor="expected-result-and-status"
    />
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
