<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { libraryCurrentPreview, libraryPreview } from '@/entities/rule-template/model/library-preview.ts';
import { libraryTarget } from '@/entities/rule-template/model/library-contracts.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import type { ReviewRow } from '../../review-shared/model/review-rows.ts';
import ReviewComparisonTable from '../../review-shared/ui/ReviewComparisonTable.vue';

/**
 * The one review of an Owner library change. It is mounted once by the app shell, so an
 * import started from any page shows this dialog, and navigating keeps the pending
 * review: the request settles only when the owner confirms or discards it.
 */
const { t } = useI18n(); const journal = libraryJournal(useControlClient());
const busy = computed(() => journal.busy.value || journal.gate.busy.value);
const draft = journal.draft;
const reviewOpen = computed({ get: () => !!draft.value, set: (value) => { if (!value) journal.dismiss(); } });
const planned = computed(() => draft.value ? libraryPreview(draft.value.context, draft.value.mutation) : []);
const current = computed(() => draft.value && journal.comparison.value ? libraryCurrentPreview(draft.value.context, journal.comparison.value, draft.value.mutation) : []);
const changedTemplates = computed(() => {
  const mutation = draft.value?.mutation;
  if (!mutation) return [];
  if (mutation.operation === 'delete-template') return draft.value!.context.catalog.templates.filter((entry) => entry.id === mutation.id).map((before) => ({ before, after: null }));
  if (mutation.operation === 'import-templates' || mutation.operation === 'apply-template' || mutation.operation === 'import-profile') return (mutation.templates ?? []).map((after) => ({ before: draft.value!.context.catalog.templates.find((entry) => entry.id === after.id) ?? null, after }));
  return [];
});
const descriptor = computed(() => {
  const mutation = draft.value?.mutation;
  if (!mutation) return '';
  if (mutation.operation === 'create-profile') return mutation.name;
  if (mutation.operation === 'import-profile') return mutation.profile.name;
  if (mutation.operation === 'import-templates') return mutation.templates.map((entry) => typeof entry.title === 'string' ? entry.title : entry.title.default).join(', ');
  const id = mutation.operation === 'assign-record' ? mutation.profileId : libraryTarget(mutation);
  return draft.value!.context.catalog.packs.find((pack) => pack.id === id)?.name ?? id;
});

/** One row per planned record, with a stable id so hover/tap rename works. */
const plannedRows = computed<ReviewRow[]>(() => planned.value.map((row, index) => ({
  id: `planned-${index}`,
  name: row.name,
  kind: row.kind,
  change: row.after === null ? 'remove' : row.before === null ? 'add' : 'change',
  before: row.before,
  after: row.after,
  editable: row.before === null && row.after !== null,
})));

function titleOf(value: { title?: unknown; id?: string } | null | undefined): string {
  const title = value?.title;
  if (typeof title === 'string') return title;
  if (title && typeof title === 'object' && 'default' in title && typeof (title as { default?: unknown }).default === 'string') {
    return (title as { default: string }).default;
  }
  return value?.id ?? '';
}

const templateRows = computed<ReviewRow[]>(() => changedTemplates.value.map((entry, index) => ({
  id: `template-${index}`,
  name: titleOf(entry.after ?? entry.before),
  change: entry.after === null ? 'remove' : entry.before === null ? 'add' : 'change',
  before: entry.before,
  after: entry.after,
})));

const currentRows = computed<ReviewRow[]>(() => {
  const comparison = journal.comparison.value;
  if (!comparison) return [];
  const rows: ReviewRow[] = [];
  for (const pack of comparison.catalog.packs) {
    rows.push({ id: `pack-${pack.id}`, name: pack.name, kind: 'event', change: 'same', after: pack });
  }
  for (const template of comparison.catalog.templates) {
    const title = typeof template.title === 'string' ? template.title : template.title.default;
    rows.push({ id: `template-current-${template.id}`, name: title, change: 'same', after: template });
  }
  for (const row of current.value) {
    rows.push({ id: `current-${row.name}-${rows.length}`, name: row.name, kind: row.kind, change: 'same', before: row.before });
  }
  return rows;
});

function onRename(id: string, name: string): void {
  const match = /^planned-(\d+)$/.exec(id);
  if (!match) return;
  const row = planned.value[Number(match[1])];
  if (!row?.after) return;
  journal.renameRecord(row.kind, row.after.id, name);
}
</script>

<template>
  <BaseDialog
    v-model:open="reviewOpen"
    :title="t('libraryRecovery.reviewTitle')"
    :width="900"
    :closable="!busy"
    class="library-changes"
  >
    <template v-if="draft">
      <p><strong>{{ t(`libraryRecovery.${draft.mutation.operation}`) }} · {{ descriptor }}</strong></p>
      <p v-if="draft.mutation.operation === 'create-profile'">
        {{ draft.mutation.description }}
      </p>
      <p v-if="draft.mutation.operation === 'import-profile'">
        {{ draft.mutation.profile.description }}
      </p>
      <p>{{ t('libraryRecovery.reviewLead', { events: planned.filter((row) => row.kind === 'event').length, actions: planned.filter((row) => row.kind === 'action').length }) }}</p>
      <p v-if="draft.mutation.operation === 'switch-profile'">
        {{ t('libraryRecovery.globalSwitch') }}
      </p>
      <p v-if="draft.mutation.operation === 'import-profile'">
        {{ t('libraryRecovery.reimport') }}
      </p>
      <p v-if="draft.mutation.operation === 'assign-record'">
        {{ t('libraryRecovery.membership') }}
      </p>
      <ReviewComparisonTable
        v-if="plannedRows.length"
        :rows="plannedRows"
        data-test="library-review-records"
        docs-path="user/cloud/changes.html"
        docs-anchor="review-changes-by-area"
        @rename="onRename"
      />
      <ReviewComparisonTable
        v-if="templateRows.length"
        :rows="templateRows"
      />
      <p
        v-if="draft.error"
        role="alert"
      >
        {{ draft.error }}
      </p>
      <p
        v-if="draft.reviewRequired || draft.submitted"
        role="alert"
      >
        {{ t(draft.submitted ? 'libraryRecovery.noResend' : 'libraryRecovery.changed') }}
      </p>
      <BaseButton
        v-if="draft.reviewRequired || draft.submitted"
        :disabled="busy || journal.reading.value"
        data-test="library-read-current"
        @click="journal.readForReview"
      >
        {{ t('libraryRecovery.readCurrent') }}
      </BaseButton>
      <section
        v-if="journal.comparison.value"
        data-test="library-current-review"
      >
        <h3>{{ t('libraryRecovery.currentTitle') }}</h3>
        <p>{{ t('libraryRecovery.currentCounts', { events: journal.comparison.value.definitions.events.length, actions: journal.comparison.value.definitions.actions.length }) }}</p>
        <ReviewComparisonTable
          v-if="currentRows.length"
          :rows="currentRows"
        />
        <BaseButton
          v-if="!draft.submitted"
          :disabled="busy"
          data-test="library-use-current"
          @click="journal.useReviewed"
        >
          {{ t('libraryRecovery.useCurrent') }}
        </BaseButton>
      </section>
    </template>
    <template #footer>
      <BaseButton
        variant="primary"
        :disabled="!draft || busy || draft.reviewRequired || draft.submitted"
        data-test="library-confirm"
        @click="journal.confirm"
      >
        {{ t('libraryRecovery.confirm') }}
      </BaseButton>
      <BaseButton
        :disabled="busy"
        data-test="library-dismiss"
        @click="journal.dismiss"
      >
        {{ t(draft?.submitted ? 'libraryRecovery.dismiss' : 'common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.library-changes :deep(.base-dialog__body) { display: grid; gap: 12px; min-width: 0; }
</style>
