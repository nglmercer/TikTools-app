<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { recordKey, type AutomationKind } from '@/entities/automation/model/record-versions.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import type { ProposalTarget } from '../model/contracts.ts';
import { useProposalReview } from '../model/proposal-review.ts';
import ReviewedAutomationDefinition from './ReviewedAutomationDefinition.vue';

const props = defineProps<{ target: ProposalTarget | null; hostVersions: ReadonlyMap<string, string> }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const model = useProposalReview(useControlClient());
const journal = model.journal;
const busy = computed(() => journal.busy.value || model.approvals.busy.value || model.definitions.busy.value || model.gate.busy.value || model.reading.value);
const unknownAcknowledged = ref(false);
const source = computed(() => model.review.value?.source ?? null);
const reviewVersions = computed(() => new Map(model.review.value?.versions.map((v) => [recordKey(v.scope as AutomationKind, v.id), v.revision]) ?? []));
const revision = (kind: AutomationKind, id: string): string => reviewVersions.value.get(recordKey(kind, id)) ?? '0';
const date = (value: number): string => new Date(value).toLocaleString();

watch(() => props.target, (next, previous) => {
  model.select(next);
  if (next?.kind !== previous?.kind || next?.id !== previous?.id) unknownAcknowledged.value = false;
}, { immediate: true });
watch(() => props.hostVersions, model.observe, { immediate: true });
watch([journal.state, model.approvals.state, model.definitions.state], model.observeHost);
watch(() => `${journal.state.value?.hostId}:${journal.state.value?.hostInstanceId}:${journal.state.value?.pending?.commandId}`, () => { unknownAcknowledged.value = false; });
watch(() => JSON.stringify([model.definitions.state.value, model.approvals.state.value]), () => { unknownAcknowledged.value = false; });
watch(open, (value) => { if (!value) { unknownAcknowledged.value = false; model.closeReview(); } });
onMounted(() => { void model.refreshJournals(); });
onBeforeUnmount(model.dispose);

function openReview(): void { unknownAcknowledged.value = false; open.value = true; void model.readSaved(); }
function openHistory(): void { unknownAcknowledged.value = false; open.value = true; void model.refreshJournals(); }
async function archiveUnknown(): Promise<void> {
  const pending = journal.state.value?.pending;
  if (!unknownAcknowledged.value || !pending) return;
  await model.reviewUnknown(pending.commandId);
  unknownAcknowledged.value = false;
}
defineExpose({ openReview });
</script>

<template>
  <div class="proposal-tools">
    <BaseButton
      v-if="target"
      size="sm"
      :disabled="busy"
      @click="openReview"
    >
      {{ t('automationProposal.open') }}
    </BaseButton>
    <BaseButton
      size="sm"
      :disabled="busy"
      @click="openHistory"
    >
      {{ t(journal.state.value?.pending ? 'automationProposal.pendingButton' : 'automationProposal.history') }}
    </BaseButton>
  </div>
  <BaseDialog
    v-model:open="open"
    class="proposal-dialog"
    :title="t('automationProposal.title')"
    :width="900"
  >
    <p
      v-if="model.definitions.state.value?.pending"
      role="alert"
    >
      {{ t('definitionRecovery.otherPending') }}
    </p>
    <p>{{ t('automationProposal.lead') }}</p>
    <p
      v-if="model.target.value"
      class="proposal-identity"
    >
      {{ t('automationProposal.selected', { kind: t(`automationProposal.${model.target.value.kind}`), id: model.target.value.id }) }}
    </p>
    <p v-else>
      {{ t('automationProposal.noneSelected') }}
    </p>
    <p
      v-if="model.reading.value"
      role="status"
    >
      {{ t('automationProposal.reading') }}
    </p>
    <p
      v-if="model.notice.value"
      role="alert"
    >
      {{ t(`automationProposal.${model.notice.value}`) }}
    </p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <p
      v-if="model.approvals.state.value?.pending"
      role="alert"
    >
      {{ t('automationProposal.approvalPending') }}
    </p>
    <template v-if="source && model.review.value">
      <p class="proposal-identity">
        {{ t('automationProposal.host', { id: model.review.value.hostId, instance: model.review.value.hostInstanceId }) }}
      </p>
      <p
        v-if="!model.sourcePrecisionSafe.value"
        role="alert"
        data-test="adoption-source-precision"
      >
        {{ t('automationProposal.sourcePrecision') }}
      </p>
      <ReviewedAutomationDefinition
        :title="t('automationProposal.source')"
        :definition="source"
        :revision="revision(source.kind, source.id)"
      />
      <section class="proposal-dependencies">
        <h3>{{ t('automationProposal.dependencies', { count: source.dependencies.length }) }}</h3>
        <p>{{ t('automationProposal.dependenciesLead') }}</p>
        <p v-if="!source.dependencies.length">
          {{ t('automationProposal.noDependencies') }}
        </p>
        <details
          v-for="dependency in source.dependencies"
          :key="dependency.id"
        >
          <summary>{{ dependency.record?.name ?? dependency.id }} · {{ dependency.id }}</summary>
          <ReviewedAutomationDefinition
            :title="dependency.id"
            :definition="dependency"
            :revision="revision('action', dependency.id)"
          />
        </details>
      </section>
      <p v-if="source.provenance?.authority.kind === 'owner'">
        {{ t('automationProposal.alreadyOwner') }}
      </p>
      <BaseCheckbox
        v-model="model.acknowledged.value"
        :disabled="!model.adoptEligible.value"
        data-test="adoption-acknowledge"
      >
        {{ t('automationProposal.acknowledge') }}
      </BaseCheckbox>
      <BaseButton
        :disabled="!model.canAdopt.value"
        data-test="adoption-submit"
        @click="model.adopt"
      >
        {{ t('automationProposal.adopt') }}
      </BaseButton>
    </template>

    <section class="proposal-journal">
      <h3>{{ t('automationProposal.journalTitle') }}</h3>
      <p>{{ t('automationProposal.journalLead') }}</p>
      <p
        v-if="journal.error.value"
        role="alert"
      >
        {{ journal.error.value }}
      </p>
      <BaseButton
        :disabled="busy"
        @click="model.refreshJournals"
      >
        {{ t('automationProposal.refreshJournal') }}
      </BaseButton>
      <div
        v-if="journal.state.value?.pending"
        class="proposal-pending"
        role="status"
      >
        <p>{{ t('automationProposal.pendingLead') }}</p>
        <p>{{ journal.state.value.pending.kind }} · {{ journal.state.value.pending.id }}</p>
        <p>{{ t('automationProposal.command', { id: journal.state.value.pending.commandId }) }}</p>
        <p>{{ t('automationProposal.revision', { revision: journal.state.value.pending.expectedRevision }) }}</p>
        <p>{{ t('automationProposal.resolveLead') }}</p>
        <BaseButton
          :disabled="busy"
          data-test="adoption-resolve"
          @click="model.resolveOriginal"
        >
          {{ t('automationProposal.resolve') }}
        </BaseButton>
        <BaseCheckbox
          v-model="unknownAcknowledged"
          :disabled="busy"
          data-test="adoption-unknown-acknowledge"
        >
          {{ t('automationProposal.acknowledgeUnknown') }}
        </BaseCheckbox>
        <BaseButton
          :disabled="busy || !unknownAcknowledged"
          data-test="adoption-review-unknown"
          @click="archiveUnknown"
        >
          {{ t('automationProposal.reviewUnknown') }}
        </BaseButton>
      </div>
      <p v-if="journal.state.value && !journal.state.value.recent.length">
        {{ t('automationProposal.emptyHistory') }}
      </p>
      <ol
        v-if="journal.state.value?.recent.length"
        class="proposal-history"
      >
        <li
          v-for="entry in journal.state.value.recent"
          :key="entry.intent.commandId"
        >
          <strong>{{ t(`automationProposal.${entry.outcome}`) }}</strong>
          <p>{{ entry.intent.kind }} · {{ entry.intent.id }}</p>
          <p>{{ t('automationProposal.command', { id: entry.intent.commandId }) }}</p>
          <p v-if="entry.revision">
            {{ t('automationProposal.revision', { revision: entry.revision }) }}
          </p>
          <p v-if="entry.errorCode">
            {{ t('automationProposal.errorCode', { code: entry.errorCode }) }}
          </p>
          <p>{{ t('automationProposal.recorded', { at: date(entry.recordedAt) }) }}</p>
        </li>
      </ol>
    </section>
    <template #footer>
      <BaseButton
        :disabled="!model.target.value || busy"
        @click="model.readSaved"
      >
        {{ t('automationProposal.read') }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.close') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.proposal-tools { display: flex; flex-wrap: wrap; gap: var(--space-2); min-width: 0; }
.proposal-tools :deep(.base-button), .proposal-dialog :deep(.base-button) { height: auto; min-height: var(--control-height-sm); white-space: normal; }
.proposal-dialog p, .proposal-dialog h3, .proposal-dialog summary, .proposal-identity { overflow-wrap: anywhere; }
.proposal-dialog :deep(.base-dialog__title) { white-space: normal; overflow-wrap: anywhere; }
.proposal-dialog :deep(.base-dialog__body) { overflow-anchor: none; }
.proposal-dialog :deep(.base-checkbox) { align-items: flex-start; margin: var(--space-3) 0; }
.proposal-dialog :deep(.base-checkbox__box) { margin-top: 3px; }
.proposal-dependencies, .proposal-journal { margin-top: var(--space-5); padding-top: var(--space-3); border-top: var(--border-width) solid var(--color-border); }
.proposal-dependencies details { margin: var(--space-3) 0; padding: var(--space-3); border: var(--border-width) solid var(--color-border); }
.proposal-dependencies summary { cursor: pointer; }
.proposal-pending { display: flex; flex-direction: column; align-items: flex-start; margin-top: var(--space-3); padding: var(--space-3); border: var(--border-width) solid var(--color-border-strong); }
.proposal-history { padding-inline-start: var(--space-5); }
.proposal-history li { padding-block: var(--space-3); overflow-wrap: anywhere; }
.proposal-history p { margin: var(--space-2) 0; }
@media (width < 600px) {
  .proposal-dialog :deep(.base-dialog__footer) { flex-direction: column; align-items: stretch; }
  .proposal-dialog :deep(.base-dialog__spacer) { display: none; }
  .proposal-dialog :deep(.base-dialog__body) { padding: var(--space-3); }
  .proposal-tools { width: 100%; }
  .proposal-tools :deep(.base-button) { flex: 1 1 140px; }
}
</style>
