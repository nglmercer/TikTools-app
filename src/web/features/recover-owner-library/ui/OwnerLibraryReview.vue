<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { libraryCurrentPreview, libraryPreview } from '@/entities/rule-template/model/library-preview.ts';
import { libraryTarget } from '@/entities/rule-template/model/library-contracts.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import OwnerLibraryChanges from './OwnerLibraryChanges.vue';

/**
 * The one review of an Owner library change. It is mounted once by the app shell, so an
 * import started from any page shows this dialog, and navigating keeps the pending
 * review: the request settles only when the owner confirms or discards it.
 */
const { t } = useI18n(); const journal = libraryJournal(useControlClient());
const historyPanel = ref<{ openHistory: () => void } | null>(null);
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
      <ol
        class="library-changes__rows"
        data-test="library-review-records"
      >
        <li
          v-for="(row, index) in planned"
          :key="index"
        >
          <BaseTextField
            v-if="row.before === null && row.after"
            :model-value="row.name"
            :aria-label="t('automations.editor.name')"
            data-test="library-record-name"
            :data-kind="row.kind"
            :data-id="row.after.id"
            @update:model-value="journal.renameRecord(row.kind, row.after!.id, $event)"
          />
          <strong v-else>{{ row.name }}</strong> · {{ t(`definitionRecovery.${row.kind}`) }} · {{ t(row.after === null ? 'libraryRecovery.remove' : row.before === null ? 'libraryRecovery.add' : 'libraryRecovery.change') }}
          <details>
            <summary>{{ t('libraryRecovery.source') }}</summary>
            <BaseJsonDiff
              v-if="row.before && row.after"
              :before="row.before"
              :after="row.after"
              :context="2"
              max-height="280px"
            />
            <BaseJsonView
              v-else
              :value="row.after ?? row.before"
              max-height="280px"
            />
          </details>
        </li>
      </ol>
      <ol class="library-changes__rows">
        <li
          v-for="(entry, index) in changedTemplates"
          :key="index"
        >
          <strong>{{ (entry.after ?? entry.before)?.id }}</strong>
          <details>
            <summary>{{ t('libraryRecovery.source') }}</summary>
            <BaseJsonDiff
              v-if="entry.before && entry.after"
              :before="entry.before"
              :after="entry.after"
              :context="2"
              max-height="280px"
            />
            <BaseJsonView
              v-else
              :value="entry.after ?? entry.before"
              max-height="280px"
            />
          </details>
        </li>
      </ol>
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
        <ul>
          <li
            v-for="pack in journal.comparison.value.catalog.packs"
            :key="pack.id"
          >
            <strong>{{ pack.name }}</strong> · {{ pack.eventIds.length }} / {{ pack.actionIds.length }}<details>
              <summary>{{ t('libraryRecovery.source') }}</summary>
              <BaseJsonView
                :value="pack"
                max-height="280px"
              />
            </details>
          </li>
        </ul>
        <ul>
          <li
            v-for="template in journal.comparison.value.catalog.templates"
            :key="template.id"
          >
            <strong>{{ typeof template.title === 'string' ? template.title : template.title.default }}</strong><details>
              <summary>{{ t('libraryRecovery.source') }}</summary>
              <BaseJsonView
                :value="template"
                max-height="280px"
              />
            </details>
          </li>
        </ul>
        <ul>
          <li
            v-for="(row, index) in current"
            :key="index"
          >
            <strong>{{ row.name }}</strong>
            <BaseJsonView
              :value="row.before"
              max-height="280px"
            />
          </li>
        </ul>
        <BaseButton
          v-if="!draft.submitted"
          :disabled="busy"
          data-test="library-use-current"
          @click="journal.useReviewed"
        >
          {{ t('libraryRecovery.useCurrent') }}
        </BaseButton>
      </section>
      <BaseButton
        v-if="draft.submitted"
        :disabled="busy"
        @click="historyPanel?.openHistory()"
      >
        {{ t('libraryRecovery.history') }}
      </BaseButton>
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
  <!-- Only a submitted change whose outcome is unknown points to the history, so it is mounted for that case. -->
  <OwnerLibraryChanges
    v-if="draft?.submitted"
    ref="historyPanel"
    hide-triggers
  />
</template>

<style scoped>
.library-changes :deep(.base-dialog__body) { display: grid; gap: 12px; min-width: 0; }
.library-changes p, .library-changes li { overflow-wrap: anywhere; }
.library-changes__rows { display: grid; gap: 12px; padding-inline-start: 22px; }
</style>
