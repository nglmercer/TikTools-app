<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import DialogRefreshBar from '@/shared/ui/dialog/DialogRefreshBar.vue';
import type { ApprovalReviewModel } from '../model/approval-review.ts';
const props = defineProps<{
  model: Pick<ApprovalReviewModel, 'journal' | 'adoption' | 'unknownAcknowledged'>;
  busy: boolean;
}>();
const emit = defineEmits<{ refresh: []; resolve: []; archiveUnknown: []; acknowledgeUnknown: [value: boolean] }>();
const { t } = useI18n();
const journal = props.model.journal;
const date = (value: number): string => new Date(value).toLocaleString();
</script>

<template>
  <section
    class="approval-section"
    data-test="approval-journal"
  >
    <h3 class="approval-text">
      {{ t('automationApproval.journalTitle') }}
    </h3>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.journalLead') }}
    </p>
    <p
      v-if="journal.error.value"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ journal.error.value }}
    </p>
    <p
      v-if="model.adoption.error.value"
      class="approval-text approval-paragraph"
      role="alert"
    >
      {{ model.adoption.error.value }}
    </p>
    <DialogRefreshBar
      :label="t('automationApproval.refreshJournal')"
      :busy="busy"
      @refresh="emit('refresh')"
    />
    <div
      v-if="journal.state.value?.pending"
      class="approval-card"
      role="status"
      data-test="approval-pending"
    >
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.pendingLead') }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.operation', { operation: t(`automationApproval.${journal.state.value.pending.operation}Operation`) }) }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ journal.state.value.pending.kind }} · {{ journal.state.value.pending.id }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ t('automationProposal.command', { id: journal.state.value.pending.commandId }) }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ t('automationProposal.revision', { revision: journal.state.value.pending.expectedRevision }) }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.resolveLead') }}
      </p>
      <BaseButton
        :disabled="busy"
        data-test="approval-resolve"
        @click="emit('resolve')"
      >
        {{ t('automationApproval.resolve') }}
      </BaseButton>
      <BaseCheckbox
        :model-value="model.unknownAcknowledged.value"
        :disabled="busy"
        data-test="approval-unknown-acknowledge"
        @update:model-value="emit('acknowledgeUnknown', $event)"
      >
        {{ t('automationApproval.acknowledgeUnknown') }}
      </BaseCheckbox>
      <BaseButton
        :disabled="busy || !model.unknownAcknowledged.value"
        data-test="approval-review-unknown"
        @click="emit('archiveUnknown')"
      >
        {{ t('automationApproval.reviewUnknown') }}
      </BaseButton>
    </div>
    <p
      v-if="journal.state.value && !journal.state.value.recent.length"
      class="approval-text approval-paragraph"
    >
      {{ t('automationApproval.emptyHistory') }}
    </p>
    <ol
      v-if="journal.state.value?.recent.length"
      class="approval-history"
    >
      <li
        v-for="entry in journal.state.value.recent"
        :key="entry.intent.commandId"
        class="approval-list-item"
      >
        <strong>{{ t(`automationApproval.${entry.outcome}`) }}</strong>
        <p class="approval-text approval-paragraph">
          {{ t('automationApproval.operation', { operation: t(`automationApproval.${entry.intent.operation}Operation`) }) }}
        </p>
        <p class="approval-text approval-paragraph">
          {{ entry.intent.kind }} · {{ entry.intent.id }}
        </p>
        <p class="approval-text approval-paragraph">
          {{ t('automationProposal.command', { id: entry.intent.commandId }) }}
        </p>
        <template v-if="entry.receipt">
          <p class="approval-text approval-paragraph">
            {{ t('automationProposal.revision', { revision: entry.receipt.revision }) }}
          </p>
          <p class="approval-text approval-paragraph">
            {{ t('automationApproval.approvalGeneration', { generation: entry.receipt.approvalGeneration }) }}
          </p>
          <p
            v-if="entry.receipt.manifestDigest"
            class="approval-identity approval-text approval-paragraph"
          >
            {{ t('automationApproval.retainedDigest', { digest: entry.receipt.manifestDigest }) }}
          </p>
        </template>
        <p
          v-if="entry.errorCode"
          class="approval-text approval-paragraph"
        >
          {{ t('automationProposal.errorCode', { code: entry.errorCode }) }}
        </p>
        <p class="approval-text approval-paragraph">
          {{ t('automationProposal.recorded', { at: date(entry.recordedAt) }) }}
        </p>
      </li>
    </ol>
  </section>
</template>
