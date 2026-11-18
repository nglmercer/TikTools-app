<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ReviewRow } from '../model/review-rows.ts';
import { reviewJson } from '../model/review-json.ts';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import ReviewDocsLink from './ReviewDocsLink.vue';

/** The full source of one comparison row. The JSON diff/view components are
 * reused here instead of being squeezed into a table column. */
const props = withDefaults(defineProps<{
  row: ReviewRow | null;
  docsPath?: string;
  docsAnchor?: string;
}>(), { docsPath: 'developer/components/import-review.html', docsAnchor: 'import-and-review' });

const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const tr = (key: string, params?: Record<string, unknown>): string => String(t(`jsonView.reviewTable.${key}`, params ?? {}));

const before = computed(() => reviewJson(props.row?.before));
const after = computed(() => reviewJson(props.row?.after));
const isDiff = computed(() => before.value !== undefined && after.value !== undefined);
const title = computed(() => props.row?.name ?? tr('source'));
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="review-source-dialog"
    :title="title"
    :width="960"
  >
    <div class="review-source-dialog__meta">
      <span
        v-if="row?.kind"
        class="review-source-dialog__chip"
      >{{ tr(row.kind) }}</span>
      <span
        v-if="row?.change"
        class="review-source-dialog__chip"
      >{{ tr(row.change === 'change' ? 'changeShort' : row.change) }}</span>
      <ReviewDocsLink
        v-if="docsPath"
        :path="docsPath"
        :anchor="docsAnchor"
      />
    </div>
    <BaseJsonDiff
      v-if="isDiff"
      :before="before"
      :after="after"
      :label="t('jsonView.diffRegion')"
      switchable
      max-height="60vh"
    />
    <BaseJsonView
      v-else
      :value="after ?? before ?? null"
      max-height="60vh"
    />
  </BaseDialog>
</template>

<style scoped>
.review-source-dialog__meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}
.review-source-dialog__chip {
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  font-size: var(--font-size-xs);
}
@media (width < 700px) {
  .review-source-dialog :deep(.base-dialog__body) { padding: var(--space-3); }
}
</style>
