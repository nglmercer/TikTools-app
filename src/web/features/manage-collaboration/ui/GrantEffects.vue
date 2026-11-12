<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { useGrantCapabilities } from '../model/use-grant-capabilities.ts';
import type { EffectGrants } from '@/entities/collaboration/api/types.ts';
import { effectCaps, effectLabels } from '../model/grant-options.ts';
import { cloneGrant } from '../model/clone-grant.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import EffectGrantEditor from './EffectGrantEditor.vue';
defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const { requiredByDelegation, setCapability } = useGrantCapabilities(model);
function setEffects(effects: EffectGrants | undefined): void { const value = cloneGrant(model.value); if (effects) value.effects = effects; else delete value.effects; model.value = value; }

</script>

<template>
  <section
    class="grant-editor__panel"
  >
    <h3>{{ t('collaboration.effectRules') }}</h3>
    <BaseSwitch
      v-for="capability in effectCaps"
      :key="capability"
      :model-value="model.capabilities.includes(capability)"
      :disabled="disabled || requiredByDelegation(capability)"
      @update:model-value="setCapability(capability, $event)"
    >
      {{ t(`collaboration.${effectLabels[capability]}`) }}
    </BaseSwitch>
    <EffectGrantEditor
      :model-value="model.effects"
      :disabled="disabled"
      @update:model-value="setEffects"
    />
  </section>
</template>
