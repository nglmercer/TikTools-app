<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { libraryCurrentPreview, libraryPreview } from '@/entities/rule-template/model/library-preview.ts';
import { libraryTarget } from '@/entities/rule-template/model/library-contracts.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import DialogRefreshBar from '@/shared/ui/dialog/DialogRefreshBar.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{ selectionKey: string; hideTriggers?: boolean }>();
const emit = defineEmits<{ recovered: [] }>();
const { t } = useI18n(); const journal = libraryJournal(useControlClient());
const history = ref(false); const acknowledged = ref(false);
const busy = computed(() => journal.busy.value || journal.gate.busy.value);
const hasPending = computed(() => !!journal.state.value?.pending);
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
watch(() => JSON.stringify([props.selectionKey, journal.state.value]), () => { acknowledged.value = false; }, { flush: 'sync' });
watch(journal.gate.owner, (owner) => { if (owner && owner !== 'library') acknowledged.value = false; }, { flush: 'sync' });
watch(journal.recoveryEpoch, () => emit('recovered'));
onMounted(() => { void journal.refresh(); });
function openHistory(): void { history.value = true; acknowledged.value = false; void journal.refresh(); }
async function archive(): Promise<void> { const pending = journal.state.value?.pending; if (pending && acknowledged.value) await journal.reviewUnknown(pending.commandId, () => acknowledged.value); }
defineExpose({ openHistory, busy, hasPending });
</script>

<template>
  <BaseButton
    v-if="!hideTriggers"
    size="sm"
    :disabled="busy"
    data-test="library-history"
    @click="openHistory"
  >
    {{ t(hasPending ? 'libraryRecovery.pendingButton' : 'libraryRecovery.history') }}
  </BaseButton>
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
          <details><summary>{{ t('libraryRecovery.source') }}</summary><pre>{{ JSON.stringify({ before: row.before, after: row.after }, null, 2) }}</pre></details>
        </li>
      </ol>
      <ol class="library-changes__rows">
        <li
          v-for="(entry, index) in changedTemplates"
          :key="index"
        >
          <strong>{{ (entry.after ?? entry.before)?.id }}</strong>
          <details><summary>{{ t('libraryRecovery.source') }}</summary><pre>{{ JSON.stringify(entry, null, 2) }}</pre></details>
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
            <strong>{{ pack.name }}</strong> · {{ pack.eventIds.length }} / {{ pack.actionIds.length }}<details><summary>{{ t('libraryRecovery.source') }}</summary><pre>{{ JSON.stringify(pack, null, 2) }}</pre></details>
          </li>
        </ul>
        <ul>
          <li
            v-for="template in journal.comparison.value.catalog.templates"
            :key="template.id"
          >
            <strong>{{ typeof template.title === 'string' ? template.title : template.title.default }}</strong><details><summary>{{ t('libraryRecovery.source') }}</summary><pre>{{ JSON.stringify(template, null, 2) }}</pre></details>
          </li>
        </ul>
        <ul>
          <li
            v-for="(row, index) in current"
            :key="index"
          >
            <strong>{{ row.name }}</strong><pre>{{ JSON.stringify(row.before, null, 2) }}</pre>
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
        @click="openHistory"
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
  <BaseDialog
    v-model:open="history"
    :title="t('libraryRecovery.title')"
    :width="900"
    class="library-changes"
  >
    <p class="library-changes__lead">
      {{ t('libraryRecovery.lead') }}
    </p>
    <DialogRefreshBar
      :label="t('definitionRecovery.refresh')"
      :busy="busy"
      @refresh="journal.refresh"
    />
    <p
      v-if="journal.unavailable.value"
      role="alert"
    >
      {{ t('libraryRecovery.unavailable') }}
    </p>
    <p
      v-if="journal.error.value"
      role="alert"
    >
      {{ journal.error.value }}
    </p>
    <section
      v-if="journal.state.value?.pending"
      data-test="library-pending"
    >
      <p>{{ t(`libraryRecovery.${journal.state.value.pending.operation}`) }} · {{ journal.state.value.pending.target }}</p>
      <p>{{ journal.state.value.pending.commandId }}</p>
      <BaseButton
        :disabled="busy"
        data-test="library-resolve"
        @click="journal.resolve"
      >
        {{ t('definitionRecovery.resolve') }}
      </BaseButton>
      <BaseCheckbox
        v-model="acknowledged"
        :disabled="busy"
        data-test="library-unknown-consent"
      >
        {{ t('definitionRecovery.unknown') }}
      </BaseCheckbox>
      <BaseButton
        :disabled="busy || !acknowledged"
        data-test="library-archive"
        @click="archive"
      >
        {{ t('definitionRecovery.archive') }}
      </BaseButton>
    </section>
    <p v-else>
      {{ t('libraryRecovery.noPending') }}
    </p>
    <ol class="library-changes__rows">
      <li
        v-for="record in journal.state.value?.recent ?? []"
        :key="record.intent.commandId"
        data-test="library-receipt"
      >
        <p>{{ t(`libraryRecovery.${record.intent.operation}`) }} · {{ record.intent.target }} · {{ t(`definitionRecovery.${record.outcome}`) }}</p>
        <p>{{ record.intent.commandId }}</p>
        <p>{{ t('definitionRecovery.base', { revision: record.intent.expectedRevision }) }}</p>
        <p>{{ t('definitionRecovery.originalInstance', { instance: record.intent.hostInstanceId }) }}</p>
        <p v-if="record.receipt">
          {{ t('definitionRecovery.revision', { revision: record.receipt.revision, count: record.receipt.affectedResources }) }}
        </p>
        <p v-if="record.errorCode">
          {{ record.errorCode }}
        </p>
      </li>
    </ol>
    <p class="library-changes__lead">
      {{ t('libraryRecovery.noResend') }}
    </p>
  </BaseDialog>
</template>

<style scoped>
.library-changes :deep(.base-dialog__body) { display: grid; gap: 12px; min-width: 0; }
.library-changes__lead { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.library-changes p, .library-changes li { overflow-wrap: anywhere; }
.library-changes pre { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 280px; overflow: auto; }
.library-changes__rows { display: grid; gap: 12px; padding-inline-start: 22px; }
</style>
