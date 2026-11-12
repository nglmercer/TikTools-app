<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { recordKey, type AutomationKind } from '@/entities/automation/model/record-versions.ts';
import ReviewedAutomationDefinition from '@/features/review-automation-proposal/ui/ReviewedAutomationDefinition.vue';
import type { CapturedApprovalReview } from '../model/contracts.ts';
const props = defineProps<{
  review: CapturedApprovalReview;
  versions: ReadonlyMap<string, string>;
  sourcePrecisionSafe: boolean;
  finalDependencyState: (id: string) => boolean | null;
}>();
const { t } = useI18n();
const source = computed(() => props.review.source);
const customTrigger = computed(() => {
  const trigger = source.value?.kind === 'event' ? source.value.record?.['trigger'] : null;
  return typeof trigger === 'string' && !trigger.startsWith('tiktok.') ? trigger : null;
});
const hasScript = computed(() => props.review?.manifest?.effects.some((effect) => effect.kind === 'script') ?? false);
const revision = (kind: AutomationKind, id: string): string => props.versions.get(recordKey(kind, id)) ?? '0';
const eligibilityKey = computed(() => ({ 'definition-missing': 'definitionMissing', 'no-delegated-subjects': 'noDelegatedSubjects',
  'subjects-unavailable': 'subjectsUnavailable', 'policy-unavailable': 'policyUnavailable' })[props.review?.eligibility.reasonCode ?? 'policy-unavailable']);
const stateLabel = (state: boolean | null): string => t(`automationApproval.${state === null ? 'unknown' : state ? 'enabled' : 'disabled'}`);
</script>

<template>
  <p class="approval-identity approval-text approval-paragraph">
    {{ t('automationProposal.host', { id: review.hostId, instance: review.hostInstanceId }) }}
  </p>
  <p
    v-if="!sourcePrecisionSafe"
    class="approval-text approval-paragraph"
    role="alert"
    data-test="approval-source-precision"
  >
    {{ t('automationApproval.sourcePrecision') }}
  </p>
  <section
    class="approval-status"
    data-test="approval-status"
  >
    <h3 class="approval-text">
      {{ t('automationApproval.eligibility') }}
    </h3>
    <p
      class="approval-text approval-paragraph"
      :role="review.eligibility.eligible ? 'status' : 'alert'"
    >
      {{ t(review.eligibility.eligible ? 'automationApproval.eligible' : `automationApproval.${eligibilityKey}`) }}
    </p>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.state', { state: t(`automationApproval.${review.approval.state}`) }) }}
    </p>
    <p
      v-if="review.approval.state !== 'none'"
      class="approval-text approval-paragraph"
    >
      {{ t(review.approval.current ? 'automationApproval.current' : 'automationApproval.notCurrent') }}
    </p>
    <p
      v-if="review.approval.generation"
      class="approval-text approval-paragraph"
    >
      {{ t('automationApproval.approvalGeneration', { generation: review.approval.generation }) }}
    </p>
    <p
      v-if="review.approval.manifestDigest"
      class="approval-identity approval-text approval-paragraph"
    >
      {{ t('automationApproval.retainedDigest', { digest: review.approval.manifestDigest }) }}
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
    class="approval-text approval-paragraph"
    data-test="approval-scoped-trigger"
  >
    {{ t('automationApproval.scopedTrigger', { topic: customTrigger }) }}
  </p>
  <p
    v-if="hasScript"
    class="approval-text approval-paragraph"
    data-test="approval-script-policy"
  >
    {{ t('automationApproval.scriptPolicy') }}
  </p>

  <section class="approval-section">
    <h3 class="approval-text">
      {{ t('automationProposal.dependencies', { count: source.dependencies.length }) }}
    </h3>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.planLead') }}
    </p>
    <p
      v-if="!source.dependencies.length"
      class="approval-text approval-paragraph"
    >
      {{ t('automationProposal.noDependencies') }}
    </p>
    <details
      v-for="dependency in source.dependencies"
      :key="dependency.id"
      class="approval-card"
    >
      <summary class="approval-text approval-summary">
        {{ dependency.record?.name ?? dependency.id }} · {{ dependency.id }} · {{ stateLabel(finalDependencyState(dependency.id)) }}
      </summary>
      <ReviewedAutomationDefinition
        :data-test="`approval-dependency-source-${dependency.id}`"
        :title="dependency.id"
        :definition="dependency"
        :revision="revision('action', dependency.id)"
      />
    </details>
  </section>
</template>
