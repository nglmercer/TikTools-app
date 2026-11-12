<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { useGrantCapabilities } from '../model/use-grant-capabilities.ts';
import { capabilityGroups, labels } from '../model/grant-options.ts';
import { cloneGrant } from '../model/clone-grant.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const { requiredByDelegation, setCapability } = useGrantCapabilities(model);
const enabledIn = (ids: readonly string[]): number => ids.filter(id => (model.value.capabilities as string[]).includes(id)).length;
function setPointsLimit(event: Event): void {
  const input = event.target as HTMLInputElement;
  const value = cloneGrant(model.value);
  value.maxPointsDelta = input.validity.badInput ? 0 : input.value === '' ? null : Number.isFinite(input.valueAsNumber) ? input.valueAsNumber : 0;
  model.value = value;
}

</script>

<template>
  <section
    class="grant-editor__panel"
  >
    <div
      v-for="group in capabilityGroups"
      :key="group.key"
      class="grant-editor__group"
    >
      <h4 class="grant-editor__group-title">
        <span>{{ t(`collaboration.grantGroups.${group.key}`) }}</span>
        <span class="grant-editor__count">{{ t('collaboration.groupCount', { on: enabledIn(group.ids), total: group.ids.length }) }}</span>
      </h4>
      <div class="grant-editor__switches">
        <BaseSwitch
          v-for="capability in group.ids"
          :key="capability"
          :model-value="model.capabilities.includes(capability)"
          :disabled="disabled || requiredByDelegation(capability)"
          @update:model-value="setCapability(capability, $event)"
        >
          {{ t(`collaboration.${labels[capability]}`) }}
        </BaseSwitch>
      </div>
    </div>
    <label
      v-if="model.capabilities.includes('points.adjust')"
      class="grant-editor__points-limit"
    >
      {{ t('collaboration.pointsLimit') }}
      <input
        type="number"
        min="0"
        step="any"
        :value="model.maxPointsDelta ?? ''"
        :disabled="disabled"
        @input="setPointsLimit"
      >
      <span>{{ t('collaboration.pointsLimitHint') }}</span>
    </label>
    <p class="grant-editor__hint">
      {{ t('collaboration.reusableGrantHint') }}
    </p>
  </section>
</template>
