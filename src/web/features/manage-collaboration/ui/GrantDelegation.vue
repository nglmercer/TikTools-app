<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { useGrantCapabilities } from '../model/use-grant-capabilities.ts';
import { delegatedEffects } from '../model/grant-options.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const { setDelegatedEffect, setOwnerEdits, setOwnerSource, setOtherAuthorEdits, setOtherAuthorSource, setDynamicTargets } = useGrantCapabilities(model);
</script>

<template>
  <section
    class="grant-editor__panel"
  >
    <h3>{{ t('collaboration.delegationTitle') }}</h3>
    <p>{{ t('collaboration.delegationHint') }}</p>
    <p>{{ t('collaboration.delegationReadHint') }}</p>
    <BaseSwitch
      v-for="effect in delegatedEffects"
      :key="effect"
      :model-value="model.automationDelegation?.effects.includes(effect) ?? false"
      :disabled="disabled"
      @update:model-value="setDelegatedEffect(effect, $event)"
    >
      {{ t(`collaboration.delegationEffects.${effect}`) }}
    </BaseSwitch>
    <BaseSwitch
      :model-value="model.automationDelegation?.allowOwnerEdits ?? false"
      :disabled="disabled || !model.automationDelegation"
      @update:model-value="setOwnerEdits"
    >
      {{ t('collaboration.delegationOwnerEdits') }}
    </BaseSwitch>
    <p>{{ t('collaboration.delegationOwnerEditsHint') }}</p>
    <BaseSwitch
      :model-value="model.automationDelegation?.allowOwnerSource ?? false"
      :disabled="disabled || !model.automationDelegation?.allowOwnerEdits"
      @update:model-value="setOwnerSource"
    >
      {{ t('collaboration.delegationOwnerSource') }}
    </BaseSwitch>
    <p>{{ t('collaboration.delegationOwnerSourceHint') }}</p>
    <BaseSwitch
      :model-value="model.automationDelegation?.allowOtherAuthorEdits ?? false"
      :disabled="disabled || !model.automationDelegation"
      @update:model-value="setOtherAuthorEdits"
    >
      {{ t('collaboration.delegationOtherEdits') }}
    </BaseSwitch>
    <p>{{ t('collaboration.delegationOtherEditsHint') }}</p>
    <BaseSwitch
      :model-value="model.automationDelegation?.allowOtherAuthorSource ?? false"
      :disabled="disabled || !model.automationDelegation?.allowOtherAuthorEdits"
      @update:model-value="setOtherAuthorSource"
    >
      {{ t('collaboration.delegationOtherSource') }}
    </BaseSwitch>
    <p>{{ t('collaboration.delegationOtherSourceHint') }}</p>
    <BaseSwitch
      :model-value="model.automationDelegation?.allowDynamicTargets ?? false"
      :disabled="disabled || !model.automationDelegation"
      @update:model-value="setDynamicTargets"
    >
      {{ t('collaboration.delegationDynamic') }}
    </BaseSwitch>
  </section>
</template>
