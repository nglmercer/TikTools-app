<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import type { CapturedApprovalReview } from '../model/contracts.ts';
import type { ApprovalReviewModel } from '../model/approval-review.ts';
const props = defineProps<{
  review: CapturedApprovalReview;
  model: Pick<ApprovalReviewModel, 'enablePlan' | 'activateEligible' | 'acknowledged' | 'canActivate' |
    'revokeAcknowledged' | 'revokeEligible' | 'canRevoke' | 'finalDependencyState'>;
}>();
const emit = defineEmits<{
  includeDependency: [id: string, include: boolean];
  setDependencyEnabled: [id: string, enabled: boolean];
  acknowledge: [value: boolean];
  acknowledgeRevoke: [value: boolean];
  activate: [];
  revoke: [];
}>();
const { t } = useI18n();
const source = computed(() => props.review.source);
const stateLabel = (state: boolean | null): string => t(`automationApproval.${state === null ? 'unknown' : state ? 'enabled' : 'disabled'}`);
</script>

<template>
  <section
    class="approval-section"
    data-test="approval-enable-plan"
  >
    <h3 class="approval-text">
      {{ t('automationApproval.plan') }}
    </h3>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.primaryEnabled', { kind: t(`automationApproval.${source.kind}`), id: source.id }) }}
    </p>
    <div
      v-for="dependency in source.dependencies"
      :key="dependency.id"
      class="approval-card"
      data-test="approval-dependency-plan"
    >
      <h4 class="approval-text approval-heading">
        {{ dependency.record?.name ?? dependency.id }} · {{ dependency.id }}
      </h4>
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.finalState', { state: stateLabel(model.finalDependencyState(dependency.id)) }) }}
      </p>
      <template v-if="dependency.record && dependency.provenance?.authority.kind === 'delegated'">
        <BaseCheckbox
          :model-value="Object.hasOwn(model.enablePlan.value, dependency.id)"
          :disabled="!model.activateEligible.value"
          :data-test="`approval-plan-include-${dependency.id}`"
          @update:model-value="emit('includeDependency', dependency.id, $event)"
        >
          {{ t('automationApproval.includeDependency') }}
        </BaseCheckbox>
        <BaseCheckbox
          v-if="Object.hasOwn(model.enablePlan.value, dependency.id)"
          :model-value="model.enablePlan.value[dependency.id]"
          :disabled="!model.activateEligible.value"
          :data-test="`approval-plan-enabled-${dependency.id}`"
          @update:model-value="emit('setDependencyEnabled', dependency.id, $event)"
        >
          {{ t('automationApproval.dependencyEnabled') }}
        </BaseCheckbox>
        <p
          v-else
          class="approval-text approval-paragraph"
        >
          {{ t('automationApproval.unchanged') }}
        </p>
      </template>
      <p
        v-else
        class="approval-text approval-paragraph"
      >
        {{ t(dependency.record ? 'automationApproval.ownerDependency' : 'automationApproval.missingDependency') }}
      </p>
    </div>
    <BaseCheckbox
      :model-value="model.acknowledged.value"
      :disabled="!model.activateEligible.value"
      data-test="approval-acknowledge"
      @update:model-value="emit('acknowledge', $event)"
    >
      {{ t('automationApproval.acknowledge') }}
    </BaseCheckbox>
    <BaseButton
      variant="primary"
      :disabled="!model.canActivate.value"
      data-test="approval-submit"
      @click="emit('activate')"
    >
      {{ t('automationApproval.activate') }}
    </BaseButton>
  </section>

  <section
    v-if="review.approval.state !== 'none'"
    class="approval-section"
  >
    <BaseCheckbox
      :model-value="model.revokeAcknowledged.value"
      :disabled="!model.revokeEligible.value"
      data-test="approval-revoke-acknowledge"
      @update:model-value="emit('acknowledgeRevoke', $event)"
    >
      {{ t('automationApproval.acknowledgeRevoke') }}
    </BaseCheckbox>
    <BaseButton
      variant="danger"
      :disabled="!model.canRevoke.value"
      data-test="approval-revoke"
      @click="emit('revoke')"
    >
      {{ t('automationApproval.revoke') }}
    </BaseButton>
  </section>
</template>
