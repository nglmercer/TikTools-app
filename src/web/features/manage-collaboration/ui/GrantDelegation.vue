<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { useGrantCapabilities } from '../model/use-grant-capabilities.ts';
import { delegatedEffects } from '../model/grant-options.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import GrantToggle from './GrantToggle.vue';
defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const { setDelegatedEffect, setOwnerEdits, setOwnerSource, setOtherAuthorEdits, setOtherAuthorSource, setDynamicTargets } = useGrantCapabilities(model);
</script>

<template>
  <section
    class="grant-editor__panel"
  >
    <div class="grant-editor__group">
      <h4 class="grant-editor__group-title">
        <span>{{ t('collaboration.delegationTitle') }}</span>
      </h4>
      <p class="grant-editor__hint">
        {{ t('collaboration.delegationHint') }}
      </p>
      <div class="grant-editor__switches">
        <BaseSwitch
          v-for="effect in delegatedEffects"
          :key="effect"
          :model-value="model.automationDelegation?.effects.includes(effect) ?? false"
          :disabled="disabled"
          @update:model-value="setDelegatedEffect(effect, $event)"
        >
          {{ t(`collaboration.delegationEffects.${effect}`) }}
        </BaseSwitch>
      </div>
      <p class="grant-editor__hint">
        {{ t('collaboration.delegationReadHint') }}
      </p>
    </div>
    <div class="grant-editor__group">
      <h4 class="grant-editor__group-title">
        <span>{{ t('collaboration.delegationEditing') }}</span>
      </h4>
      <div class="grant-editor__stack">
        <GrantToggle
          :model-value="model.automationDelegation?.allowOwnerEdits ?? false"
          :disabled="disabled || !model.automationDelegation"
          :hint="t('collaboration.delegationOwnerEditsHint')"
          @update:model-value="setOwnerEdits"
        >
          {{ t('collaboration.delegationOwnerEdits') }}
        </GrantToggle>
        <GrantToggle
          nested
          :model-value="model.automationDelegation?.allowOwnerSource ?? false"
          :disabled="disabled || !model.automationDelegation?.allowOwnerEdits"
          :hint="t('collaboration.delegationOwnerSourceHint')"
          @update:model-value="setOwnerSource"
        >
          {{ t('collaboration.delegationOwnerSource') }}
        </GrantToggle>
        <GrantToggle
          :model-value="model.automationDelegation?.allowOtherAuthorEdits ?? false"
          :disabled="disabled || !model.automationDelegation"
          :hint="t('collaboration.delegationOtherEditsHint')"
          @update:model-value="setOtherAuthorEdits"
        >
          {{ t('collaboration.delegationOtherEdits') }}
        </GrantToggle>
        <GrantToggle
          nested
          :model-value="model.automationDelegation?.allowOtherAuthorSource ?? false"
          :disabled="disabled || !model.automationDelegation?.allowOtherAuthorEdits"
          :hint="t('collaboration.delegationOtherSourceHint')"
          @update:model-value="setOtherAuthorSource"
        >
          {{ t('collaboration.delegationOtherSource') }}
        </GrantToggle>
        <GrantToggle
          :model-value="model.automationDelegation?.allowDynamicTargets ?? false"
          :disabled="disabled || !model.automationDelegation"
          @update:model-value="setDynamicTargets"
        >
          {{ t('collaboration.delegationDynamic') }}
        </GrantToggle>
      </div>
    </div>
  </section>
</template>
