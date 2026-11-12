<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import type { ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import { useApprovalReview } from '../model/approval-review.ts';

import ApprovalSourceReview from './ApprovalSourceReview.vue';
import ApprovalManifestReview from './ApprovalManifestReview.vue';
import ApprovalDecisionPanel from './ApprovalDecisionPanel.vue';
import ApprovalJournalPanel from './ApprovalJournalPanel.vue';

const props = defineProps<{ target: ProposalTarget | null; hostVersions: ReadonlyMap<string, string>; hideTriggers?: boolean }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const model = useApprovalReview(useControlClient());
const journal = model.journal;
const source = computed(() => model.review.value?.source ?? null);
const busy = computed(() => journal.busy.value || model.adoption.busy.value || model.definitions.busy.value || model.library.busy.value || model.gate.busy.value || model.reading.value);

watch(() => props.target, model.select, { immediate: true });
watch(() => props.hostVersions, model.observe, { immediate: true });
watch([journal.state, model.adoption.state, model.definitions.state, model.library.state], model.observeHost);
watch(() => `${journal.state.value?.hostId}:${journal.state.value?.hostInstanceId}:${journal.state.value?.pending?.commandId}`, () => { model.unknownAcknowledged.value = false; });
watch(open, (value) => { if (!value) model.closeReview(); });
onMounted(() => { void model.refreshJournals(); });
onBeforeUnmount(model.dispose);

function openReview(): void { model.unknownAcknowledged.value = false; open.value = true; void model.readSaved(); }
function openHistory(): void { model.unknownAcknowledged.value = false; open.value = true; void model.refreshJournals(); }
async function archiveUnknown(): Promise<void> {
  const pending = journal.state.value?.pending;
  if (pending) await model.reviewUnknown(pending.commandId);
}
const hasPending = computed(() => !!journal.state.value?.pending);
defineExpose({ openReview, openHistory, busy, hasPending });
</script>

<template>
  <div
    v-if="!hideTriggers"
    class="approval-tools"
  >
    <BaseButton
      v-if="target"
      size="sm"
      :disabled="busy"
      data-test="approval-open"
      @click="openReview"
    >
      {{ t('automationApproval.open') }}
    </BaseButton>
    <BaseButton
      size="sm"
      :disabled="busy"
      data-test="approval-history"
      @click="openHistory"
    >
      {{ t(journal.state.value?.pending ? 'automationApproval.pendingButton' : 'automationApproval.history') }}
    </BaseButton>
  </div>
  <BaseDialog
    v-model:open="open"
    class="approval-dialog"
    :title="t('automationApproval.title')"
    :width="1000"
  >
    <p
      v-if="model.library.state.value?.pending"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ t('libraryRecovery.otherPending') }}
    </p>
    <p
      v-if="model.definitions.state.value?.pending"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ t('definitionRecovery.otherPending') }}
    </p>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.lead') }}
    </p>
    <p
      v-if="model.target.value"
      class="approval-identity approval-text approval-paragraph"
    >
      {{ t('automationApproval.selected', { kind: t(`automationApproval.${model.target.value.kind}`), id: model.target.value.id }) }}
    </p>
    <p
      v-else
      class="approval-text approval-paragraph"
    >
      {{ t('automationApproval.noneSelected') }}
    </p>
    <p
      v-if="model.reading.value"
      class="approval-text approval-paragraph"
      role="status"
    >
      {{ t('automationApproval.reading') }}
    </p>
    <p
      v-if="model.notice.value"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ t(`automationApproval.${model.notice.value}`) }}
    </p>
    <p
      v-if="model.error.value"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <p
      v-if="model.adoption.state.value?.pending"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ t('automationApproval.adoptionPending') }}
    </p>

    <template v-if="source && model.review.value">
      <ApprovalSourceReview
        :review="model.review.value"
        :versions="model.reviewVersions.value"
        :source-precision-safe="model.sourcePrecisionSafe.value"
        :final-dependency-state="model.finalDependencyState"
      />
      <ApprovalManifestReview :review="model.review.value" />
      <ApprovalDecisionPanel
        :review="model.review.value"
        :model="model"
        @include-dependency="model.includeDependency"
        @set-dependency-enabled="model.setDependencyEnabled"
        @acknowledge="model.acknowledged.value = $event"
        @acknowledge-revoke="model.revokeAcknowledged.value = $event"
        @activate="model.activate"
        @revoke="model.revoke"
      />
    </template>

    <ApprovalJournalPanel
      :model="model"
      :busy="busy"
      @refresh="model.refreshJournals"
      @resolve="model.resolveOriginal"
      @archive-unknown="archiveUnknown"
      @acknowledge-unknown="model.unknownAcknowledged.value = $event"
    />
    <template #footer>
      <BaseButton
        :disabled="!model.target.value || busy"
        @click="model.readSaved"
      >
        {{ t('automationApproval.read') }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.close') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style src="./approval-review.css"></style>
