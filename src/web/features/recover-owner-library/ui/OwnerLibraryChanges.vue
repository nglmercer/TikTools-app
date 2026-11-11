<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import DialogRefreshBar from '@/shared/ui/dialog/DialogRefreshBar.vue';

/**
 * History and recovery of the Owner library. The review of a pending change is not
 * here: it lives in OwnerLibraryReview, mounted once by the app shell.
 */
const props = withDefaults(defineProps<{ selectionKey?: string; hideTriggers?: boolean }>(), { selectionKey: '' });
const emit = defineEmits<{ recovered: [] }>();
const { t } = useI18n(); const journal = libraryJournal(useControlClient());
const history = ref(false); const acknowledged = ref(false);
const busy = computed(() => journal.busy.value || journal.gate.busy.value);
const hasPending = computed(() => !!journal.state.value?.pending);
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
.library-changes__rows { display: grid; gap: 12px; padding-inline-start: 22px; }
</style>
