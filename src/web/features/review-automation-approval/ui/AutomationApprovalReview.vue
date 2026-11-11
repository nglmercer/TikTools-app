<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { recordKey, type AutomationKind } from '@/entities/automation/model/record-versions.ts';
import type { ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';
import ReviewedAutomationDefinition from '@/features/review-automation-proposal/ui/ReviewedAutomationDefinition.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import DialogRefreshBar from '@/shared/ui/dialog/DialogRefreshBar.vue';
import { useApprovalReview } from '../model/approval-review.ts';

const props = defineProps<{ target: ProposalTarget | null; hostVersions: ReadonlyMap<string, string>; hideTriggers?: boolean }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const model = useApprovalReview(useControlClient());
const journal = model.journal;
const source = computed(() => model.review.value?.source ?? null);
const customTrigger = computed(() => {
  const trigger = source.value?.kind === 'event' ? source.value.record?.['trigger'] : null;
  return typeof trigger === 'string' && !trigger.startsWith('tiktok.') ? trigger : null;
});
const hasScript = computed(() => model.review.value?.manifest?.effects.some((effect) => effect.kind === 'script') ?? false);
const revision = (kind: AutomationKind, id: string): string => model.reviewVersions.value.get(recordKey(kind, id)) ?? '0';
const date = (value: number): string => new Date(value).toLocaleString();
const eligibilityKey = computed(() => ({ 'definition-missing': 'definitionMissing', 'no-delegated-subjects': 'noDelegatedSubjects',
  'subjects-unavailable': 'subjectsUnavailable', 'policy-unavailable': 'policyUnavailable' })[model.review.value?.eligibility.reasonCode ?? 'policy-unavailable']);
const stateLabel = (state: boolean | null): string => t(`automationApproval.${state === null ? 'unknown' : state ? 'enabled' : 'disabled'}`);
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
      role="alert"
    >
      {{ t('libraryRecovery.otherPending') }}
    </p>
    <p
      v-if="model.definitions.state.value?.pending"
      role="alert"
    >
      {{ t('definitionRecovery.otherPending') }}
    </p>
    <p>{{ t('automationApproval.lead') }}</p>
    <p
      v-if="model.target.value"
      class="approval-identity"
    >
      {{ t('automationApproval.selected', { kind: t(`automationApproval.${model.target.value.kind}`), id: model.target.value.id }) }}
    </p>
    <p v-else>
      {{ t('automationApproval.noneSelected') }}
    </p>
    <p
      v-if="model.reading.value"
      role="status"
    >
      {{ t('automationApproval.reading') }}
    </p>
    <p
      v-if="model.notice.value"
      role="alert"
    >
      {{ t(`automationApproval.${model.notice.value}`) }}
    </p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <p
      v-if="model.adoption.state.value?.pending"
      role="alert"
    >
      {{ t('automationApproval.adoptionPending') }}
    </p>

    <template v-if="source && model.review.value">
      <p class="approval-identity">
        {{ t('automationProposal.host', { id: model.review.value.hostId, instance: model.review.value.hostInstanceId }) }}
      </p>
      <p
        v-if="!model.sourcePrecisionSafe.value"
        role="alert"
        data-test="approval-source-precision"
      >
        {{ t('automationApproval.sourcePrecision') }}
      </p>
      <section
        class="approval-status"
        data-test="approval-status"
      >
        <h3>{{ t('automationApproval.eligibility') }}</h3>
        <p :role="model.review.value.eligibility.eligible ? 'status' : 'alert'">
          {{ t(model.review.value.eligibility.eligible ? 'automationApproval.eligible' : `automationApproval.${eligibilityKey}`) }}
        </p>
        <p>{{ t('automationApproval.state', { state: t(`automationApproval.${model.review.value.approval.state}`) }) }}</p>
        <p v-if="model.review.value.approval.state !== 'none'">
          {{ t(model.review.value.approval.current ? 'automationApproval.current' : 'automationApproval.notCurrent') }}
        </p>
        <p v-if="model.review.value.approval.generation">
          {{ t('automationApproval.approvalGeneration', { generation: model.review.value.approval.generation }) }}
        </p>
        <p
          v-if="model.review.value.approval.manifestDigest"
          class="approval-identity"
        >
          {{ t('automationApproval.retainedDigest', { digest: model.review.value.approval.manifestDigest }) }}
        </p>
      </section>
      <ReviewedAutomationDefinition
        data-test="approval-source"
        :title="t('automationProposal.source')"
        :definition="source"
        :revision="revision(source.kind, source.id)"
      />
      <p
        v-if="customTrigger"
        data-test="approval-scoped-trigger"
      >
        {{ t('automationApproval.scopedTrigger', { topic: customTrigger }) }}
      </p>
      <p
        v-if="hasScript"
        data-test="approval-script-policy"
      >
        {{ t('automationApproval.scriptPolicy') }}
      </p>

      <section class="approval-section">
        <h3>{{ t('automationProposal.dependencies', { count: source.dependencies.length }) }}</h3>
        <p>{{ t('automationApproval.planLead') }}</p>
        <p v-if="!source.dependencies.length">
          {{ t('automationProposal.noDependencies') }}
        </p>
        <details
          v-for="dependency in source.dependencies"
          :key="dependency.id"
          class="approval-card"
        >
          <summary>{{ dependency.record?.name ?? dependency.id }} · {{ dependency.id }} · {{ stateLabel(model.finalDependencyState(dependency.id)) }}</summary>
          <ReviewedAutomationDefinition
            :data-test="`approval-dependency-source-${dependency.id}`"
            :title="dependency.id"
            :definition="dependency"
            :revision="revision('action', dependency.id)"
          />
        </details>
      </section>

      <section
        class="approval-section"
        data-test="approval-manifest"
      >
        <h3>{{ t('automationApproval.manifest') }}</h3>
        <p>{{ t('automationApproval.manifestLead') }}</p>
        <template v-if="model.review.value.manifest">
          <p>{{ t('automationApproval.policy', { version: model.review.value.manifest.policyVersion }) }}</p>
          <p class="approval-identity">
            {{ t('automationApproval.digest', { digest: model.review.value.manifest.manifestDigest }) }}
          </p>
          <h4>{{ t('automationApproval.reasons') }}</h4>
          <ul>
            <li
              v-for="reason in model.review.value.manifest.reasons"
              :key="reason"
            >
              {{ t(`automationApproval.reason.${reason}`) }}
            </li>
          </ul>
          <h4>{{ t('automationApproval.effects', { count: model.review.value.manifest.effects.length }) }}</h4>
          <article
            v-for="effect in model.review.value.manifest.effects"
            :key="effect.id"
            class="approval-card"
            data-test="approval-effect"
          >
            <h4>{{ effect.id }} · {{ effect.typeId ?? effect.kind }}</h4>
            <p>{{ t(`automationApproval.${effect.decision}`) }} · {{ stateLabel(effect.enabled) }}</p>
            <p v-if="effect.dynamicTarget">
              {{ t('automationApproval.dynamic') }}
            </p>
            <h5>{{ t('automationApproval.fields') }}</h5>
            <ul v-if="effect.renderFields.length">
              <li
                v-for="field in effect.renderFields"
                :key="field"
              >
                <code>{{ field }}</code>
              </li>
            </ul>
            <p v-else>
              {{ t('automationApproval.noFields') }}
            </p>
            <h5>{{ t('automationApproval.capabilities') }}</h5>
            <ul v-if="effect.requiredCapabilities.length">
              <li
                v-for="capability in effect.requiredCapabilities"
                :key="capability"
              >
                <code>{{ capability }}</code>
              </li>
            </ul>
            <p v-else>
              {{ t('automationApproval.noCapabilities') }}
            </p>
            <ul v-if="effect.reasons.length">
              <li
                v-for="reason in effect.reasons"
                :key="reason"
              >
                {{ t(`automationApproval.reason.${reason}`) }}
              </li>
            </ul>
          </article>
        </template>
        <p v-else>
          {{ t('automationApproval.unavailableManifest') }}
        </p>
      </section>

      <section
        class="approval-section"
        data-test="approval-subjects"
      >
        <h3>{{ t('automationApproval.subjects', { count: model.review.value.subjects.length }) }}</h3>
        <p>{{ t('automationApproval.subjectsLead') }}</p>
        <details
          v-for="subject in model.review.value.subjects"
          :key="`${subject.principalId}:${subject.sessionId}`"
          class="approval-card"
          data-test="approval-subject"
        >
          <summary>{{ subject.principalId }} · {{ subject.sessionId }}</summary>
          <p>{{ t('automationApproval.grantRevision', { revision: subject.grantRevision }) }}</p>
          <p>{{ t('automationApproval.configurationRevision', { revision: subject.configurationRevision }) }}</p>
          <pre tabindex="0">{{ JSON.stringify(subject.grant, null, 2) }}</pre>
        </details>
      </section>

      <section
        class="approval-section"
        data-test="approval-enable-plan"
      >
        <h3>{{ t('automationApproval.plan') }}</h3>
        <p>{{ t('automationApproval.primaryEnabled', { kind: t(`automationApproval.${source.kind}`), id: source.id }) }}</p>
        <div
          v-for="dependency in source.dependencies"
          :key="dependency.id"
          class="approval-card"
          data-test="approval-dependency-plan"
        >
          <h4>{{ dependency.record?.name ?? dependency.id }} · {{ dependency.id }}</h4>
          <p>{{ t('automationApproval.finalState', { state: stateLabel(model.finalDependencyState(dependency.id)) }) }}</p>
          <template v-if="dependency.record && dependency.provenance?.authority.kind === 'delegated'">
            <BaseCheckbox
              :model-value="Object.hasOwn(model.enablePlan.value, dependency.id)"
              :disabled="!model.activateEligible.value"
              :data-test="`approval-plan-include-${dependency.id}`"
              @update:model-value="model.includeDependency(dependency.id, $event)"
            >
              {{ t('automationApproval.includeDependency') }}
            </BaseCheckbox>
            <BaseCheckbox
              v-if="Object.hasOwn(model.enablePlan.value, dependency.id)"
              :model-value="model.enablePlan.value[dependency.id]"
              :disabled="!model.activateEligible.value"
              :data-test="`approval-plan-enabled-${dependency.id}`"
              @update:model-value="model.setDependencyEnabled(dependency.id, $event)"
            >
              {{ t('automationApproval.dependencyEnabled') }}
            </BaseCheckbox>
            <p v-else>
              {{ t('automationApproval.unchanged') }}
            </p>
          </template>
          <p v-else>
            {{ t(dependency.record ? 'automationApproval.ownerDependency' : 'automationApproval.missingDependency') }}
          </p>
        </div>
        <BaseCheckbox
          v-model="model.acknowledged.value"
          :disabled="!model.activateEligible.value"
          data-test="approval-acknowledge"
        >
          {{ t('automationApproval.acknowledge') }}
        </BaseCheckbox>
        <BaseButton
          variant="primary"
          :disabled="!model.canActivate.value"
          data-test="approval-submit"
          @click="model.activate"
        >
          {{ t('automationApproval.activate') }}
        </BaseButton>
      </section>

      <section
        v-if="model.review.value.approval.state !== 'none'"
        class="approval-section"
      >
        <BaseCheckbox
          v-model="model.revokeAcknowledged.value"
          :disabled="!model.revokeEligible.value"
          data-test="approval-revoke-acknowledge"
        >
          {{ t('automationApproval.acknowledgeRevoke') }}
        </BaseCheckbox>
        <BaseButton
          variant="danger"
          :disabled="!model.canRevoke.value"
          data-test="approval-revoke"
          @click="model.revoke"
        >
          {{ t('automationApproval.revoke') }}
        </BaseButton>
      </section>
    </template>

    <section
      class="approval-section"
      data-test="approval-journal"
    >
      <h3>{{ t('automationApproval.journalTitle') }}</h3>
      <p>{{ t('automationApproval.journalLead') }}</p>
      <p
        v-if="journal.error.value"
        role="alert"
      >
        {{ journal.error.value }}
      </p>
      <p
        v-if="model.adoption.error.value"
        role="alert"
      >
        {{ model.adoption.error.value }}
      </p>
      <DialogRefreshBar
        :label="t('automationApproval.refreshJournal')"
        :busy="busy"
        @refresh="model.refreshJournals"
      />
      <div
        v-if="journal.state.value?.pending"
        class="approval-card"
        role="status"
        data-test="approval-pending"
      >
        <p>{{ t('automationApproval.pendingLead') }}</p>
        <p>{{ t('automationApproval.operation', { operation: t(`automationApproval.${journal.state.value.pending.operation}Operation`) }) }}</p>
        <p>{{ journal.state.value.pending.kind }} · {{ journal.state.value.pending.id }}</p>
        <p>{{ t('automationProposal.command', { id: journal.state.value.pending.commandId }) }}</p>
        <p>{{ t('automationProposal.revision', { revision: journal.state.value.pending.expectedRevision }) }}</p>
        <p>{{ t('automationApproval.resolveLead') }}</p>
        <BaseButton
          :disabled="busy"
          data-test="approval-resolve"
          @click="model.resolveOriginal"
        >
          {{ t('automationApproval.resolve') }}
        </BaseButton>
        <BaseCheckbox
          v-model="model.unknownAcknowledged.value"
          :disabled="busy"
          data-test="approval-unknown-acknowledge"
        >
          {{ t('automationApproval.acknowledgeUnknown') }}
        </BaseCheckbox>
        <BaseButton
          :disabled="busy || !model.unknownAcknowledged.value"
          data-test="approval-review-unknown"
          @click="archiveUnknown"
        >
          {{ t('automationApproval.reviewUnknown') }}
        </BaseButton>
      </div>
      <p v-if="journal.state.value && !journal.state.value.recent.length">
        {{ t('automationApproval.emptyHistory') }}
      </p>
      <ol
        v-if="journal.state.value?.recent.length"
        class="approval-history"
      >
        <li
          v-for="entry in journal.state.value.recent"
          :key="entry.intent.commandId"
        >
          <strong>{{ t(`automationApproval.${entry.outcome}`) }}</strong>
          <p>{{ t('automationApproval.operation', { operation: t(`automationApproval.${entry.intent.operation}Operation`) }) }}</p>
          <p>{{ entry.intent.kind }} · {{ entry.intent.id }}</p>
          <p>{{ t('automationProposal.command', { id: entry.intent.commandId }) }}</p>
          <template v-if="entry.receipt">
            <p>{{ t('automationProposal.revision', { revision: entry.receipt.revision }) }}</p>
            <p>{{ t('automationApproval.approvalGeneration', { generation: entry.receipt.approvalGeneration }) }}</p>
            <p
              v-if="entry.receipt.manifestDigest"
              class="approval-identity"
            >
              {{ t('automationApproval.retainedDigest', { digest: entry.receipt.manifestDigest }) }}
            </p>
          </template>
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
        {{ t('automationApproval.read') }}
      </BaseButton>
      <BaseButton @click="open = false">
        {{ t('common.close') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.approval-tools { display: flex; flex-wrap: wrap; gap: var(--space-2); min-width: 0; }
.approval-tools :deep(.base-button), .approval-dialog :deep(.base-button) { height: auto; min-height: var(--control-height-sm); white-space: normal; }
.approval-dialog p, .approval-dialog h3, .approval-dialog h4, .approval-dialog summary, .approval-identity { overflow-wrap: anywhere; }
.approval-dialog :deep(.base-dialog__title) { white-space: normal; overflow-wrap: anywhere; }
.approval-dialog :deep(.base-dialog__body) { overflow-anchor: none; }
.approval-dialog :deep(.base-checkbox) { align-items: flex-start; margin: var(--space-3) 0; }
.approval-dialog :deep(.base-checkbox__box) { margin-top: 3px; }
.approval-status, .approval-card { margin: var(--space-3) 0; padding: var(--space-3); border: var(--border-width) solid var(--color-border); border-radius: var(--radius-sm); }
.approval-section { margin-top: var(--space-5); padding-top: var(--space-3); border-top: var(--border-width) solid var(--color-border); }
.approval-card summary { cursor: pointer; overflow-wrap: anywhere; }
.approval-card h4, .approval-card h5 { margin-block: var(--space-2); }
.approval-card pre { max-height: 28vh; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
.approval-card ul { padding-inline-start: var(--space-5); }
.approval-history { padding-inline-start: var(--space-5); }
.approval-history li { padding-block: var(--space-3); overflow-wrap: anywhere; }
.approval-history p { margin: var(--space-2) 0; }
@media (width < 600px) {
  .approval-dialog :deep(.base-dialog__footer) { flex-direction: column; align-items: stretch; }
  .approval-dialog :deep(.base-dialog__spacer) { display: none; }
  .approval-dialog :deep(.base-dialog__body) { padding: var(--space-3); }
  .approval-tools { width: 100%; }
  .approval-tools :deep(.base-button) { flex: 1 1 140px; }
}
</style>
