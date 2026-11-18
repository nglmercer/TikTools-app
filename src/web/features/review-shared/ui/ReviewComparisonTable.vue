<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ReviewRow } from '../model/review-rows.ts';
import { reviewHasValue } from '../model/review-json.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import IconCode from '@/shared/ui/icon/icons/IconCode.vue';
import ReviewDocsLink from './ReviewDocsLink.vue';
import ReviewSourceDialog from './ReviewSourceDialog.vue';

/**
 * One compact, numbered comparison table for both review dialogs. The source
 * of each row opens in its own dialog, where BaseJsonDiff/BaseJsonView get the
 * full width (and the existing unified / side-by-side switch). This replaces
 * both the repeated "Review source" text rows and squeezed inline JSON.
 */
const props = withDefaults(defineProps<{
  rows: readonly ReviewRow[];
  caption?: string;
  docsPath?: string;
  docsAnchor?: string;
  nameLabel?: string;
}>(), {
  caption: '',
  docsPath: 'developer/components/import-review.html',
  docsAnchor: 'import-and-review',
  nameLabel: '',
});

const emit = defineEmits<{ rename: [id: string, name: string] }>();
const { t } = useI18n();

function tr(key: string, params?: Record<string, unknown>): string {
  return String(t(`jsonView.reviewTable.${key}`, params ?? {}));
}

const selected = ref<ReviewRow | null>(null);
const dialogOpen = ref(false);
function showSource(row: ReviewRow): void {
  selected.value = row;
  dialogOpen.value = true;
}
function closeSource(): void {
  dialogOpen.value = false;
}

const summaryCount = computed(() => tr('records', { count: props.rows.length }));
const changeClass = (row: ReviewRow): string => `review-table__chip--${row.change ?? 'same'}`;
</script>

<template>
  <div class="review-table">
    <div
      v-if="caption || docsPath"
      class="review-table__header"
    >
      <p v-if="caption">
        {{ caption }}
      </p>
      <ReviewDocsLink
        v-if="docsPath"
        :path="docsPath"
        :anchor="docsAnchor"
      />
    </div>
    <div class="review-table__scroll">
      <table class="review-table__table">
        <caption class="review-table__sr">
          {{ caption || summaryCount }}
        </caption>
        <colgroup>
          <col style="width: 44px">
          <col>
          <col style="width: 88px">
          <col style="width: 96px">
          <col style="width: 64px">
        </colgroup>
        <thead>
          <tr>
            <th
              scope="col"
              class="review-table__index"
            >
              {{ tr('index') }}
            </th>
            <th scope="col">
              {{ tr('record') }}
            </th>
            <th scope="col">
              {{ tr('type') }}
            </th>
            <th scope="col">
              {{ tr('change') }}
            </th>
            <th scope="col">
              {{ tr('actions') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, index) in rows"
            :key="row.id || String(index)"
            class="review-table__row"
          >
            <td class="review-table__index">
              {{ index + 1 }}
            </td>
            <td class="review-table__name">
              <BaseTextField
                v-if="row.editable"
                :model-value="row.name"
                :aria-label="nameLabel || tr('record')"
                data-test="library-record-name"
                @update:model-value="emit('rename', row.id, $event)"
              />
              <strong v-else>{{ row.name }}</strong>
              <span
                v-if="row.revision"
                class="review-table__revision"
              >{{ tr('revision', { revision: row.revision }) }}</span>
              <span
                v-if="row.note"
                class="review-table__revision"
              >{{ row.note }}</span>
            </td>
            <td>
              <span
                v-if="row.kind"
                class="review-table__chip"
              >{{ tr(row.kind) }}</span>
            </td>
            <td>
              <span
                class="review-table__chip"
                :class="changeClass(row)"
              >{{ tr(row.change === 'change' ? 'changeShort' : (row.change ?? 'same')) }}</span>
            </td>
            <td>
              <BaseIconButton
                class="review-table__action"
                :label="tr('source')"
                :disabled="!reviewHasValue(row.before, row.after)"
                size="sm"
                @click="showSource(row)"
              >
                <IconCode />
              </BaseIconButton>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <ReviewSourceDialog
      v-model:open="dialogOpen"
      :row="selected"
      :docs-path="docsPath"
      :docs-anchor="docsAnchor"
      @update:open="dialogOpen || closeSource()"
    />
  </div>
</template>

<style scoped>
.review-table {
  display: grid;
  gap: var(--space-3);
  min-width: 0;
}
.review-table__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.review-table__header p { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.review-table__scroll { max-height: min(52vh, 520px); overflow: auto; }
.review-table__table { width: 100%; border-collapse: collapse; }
.review-table__table th,
.review-table__table td {
  padding: var(--space-2);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  text-align: left;
  vertical-align: top;
  overflow-wrap: anywhere;
}
.review-table__table th { color: var(--color-text-muted); font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); }
.review-table__index { width: 44px; font-variant-numeric: tabular-nums; color: var(--color-text-muted); }
.review-table__name { min-width: 160px; display: grid; gap: 2px; }
.review-table__revision { color: var(--color-text-subtle); font-size: var(--font-size-xs); }
.review-table__chip {
  display: inline-block;
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  font-size: var(--font-size-xs);
}
.review-table__chip--add { color: var(--color-success, inherit); }
.review-table__chip--remove { color: var(--color-danger, inherit); }
.review-table__chip--change,
.review-table__chip--diff { color: var(--color-warning, inherit); }
.review-table__action { opacity: 0; transition: opacity var(--duration-fast); }
.review-table__row:hover .review-table__action,
.review-table__row:focus-within .review-table__action { opacity: 1; }
.review-table__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  border: 0;
}
@media (prefers-reduced-motion: reduce) {
  .review-table__action { transition: none; }
}
@media (any-pointer: coarse), (width < 700px) {
  .review-table__scroll { max-height: 40vh; }
  .review-table__table { min-width: 420px; }
  .review-table__action { opacity: 1; }
}
</style>
