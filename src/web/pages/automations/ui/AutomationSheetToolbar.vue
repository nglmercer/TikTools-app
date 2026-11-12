<script setup lang="ts">
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import type { ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';
import type { useAutomationHistory } from '@/widgets/automation-history-menu/model/use-automation-history.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import type { EditingRecord } from '../model/editing-record.ts';
const props = defineProps<{
  editing: EditingRecord;
  automation: Pick<AutomationControls, 'persistent' | 'sourceHost'>;
  proposalTarget: ProposalTarget | null;
  history: Pick<ReturnType<typeof useAutomationHistory>, 'libraryPanel' | 'definitionRecoveryPanel' | 'proposalReviewPanel' | 'approvalReviewPanel'>;
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useAutomationText();
const { libraryPanel, definitionRecoveryPanel, proposalReviewPanel, approvalReviewPanel } = props.history;
</script>

<template>
  <header
    class="automations-page__sheet-bar"
  >
    <BaseIconButton
      :label="t('common.back')"
      variant="default"
      size="sm"
      @click="emit('close')"
    >
      <IconChevronLeft />
    </BaseIconButton>
    <span class="automations-page__sheet-title">{{ editing.record.name }}</span>
    <BaseButton
      v-if="automation.persistent.value !== false"
      size="sm"
      :disabled="!libraryPanel || libraryPanel.busy"
      data-test="library-history"
      @click="libraryPanel?.openHistory()"
    >
      {{ t(libraryPanel?.hasPending ? 'libraryRecovery.pendingButton' : 'libraryRecovery.history') }}
    </BaseButton>
    <BaseButton
      v-if="automation.persistent.value === true || automation.sourceHost.value !== null"
      size="sm"
      :disabled="!definitionRecoveryPanel || definitionRecoveryPanel.busy"
      data-test="definition-history"
      @click="definitionRecoveryPanel?.openHistory()"
    >
      {{ t(definitionRecoveryPanel?.hasPending ? 'definitionRecovery.pendingButton' : 'definitionRecovery.history') }}
    </BaseButton>
    <BaseButton
      v-if="automation.persistent.value === true && proposalTarget"
      class="automations-page__proposal-button"
      size="sm"
      @click="proposalReviewPanel?.openReview()"
    >
      {{ t('automationProposal.open') }}
    </BaseButton>
    <BaseButton
      v-if="automation.persistent.value === true && proposalTarget"
      size="sm"
      data-test="approval-open"
      @click="approvalReviewPanel?.openReview()"
    >
      {{ t('automationApproval.open') }}
    </BaseButton>
  </header>
</template>
