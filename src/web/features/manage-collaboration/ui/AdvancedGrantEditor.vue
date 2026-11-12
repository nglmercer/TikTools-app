<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { ref, watch } from 'vue';
import { capabilities } from '@/entities/collaboration/api/types.ts';
import { parseGrant } from '@/entities/collaboration/lib/grants.ts';
import { current, effectCaps } from '../model/grant-options.ts';
import { useGrantCapabilities } from '../model/use-grant-capabilities.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
const props = defineProps<{ disabled?: boolean; resetKey: number }>();
const emit = defineEmits<{ applied: [] }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const { requiredByDelegation, setCapability } = useGrantCapabilities(model);
const jsonDraft = ref('');
const jsonDirty = ref(false);
const invalid = ref(false);
watch(model, value => { if (!jsonDirty.value) jsonDraft.value = JSON.stringify(value, null, 2); }, { immediate: true, deep: true });
watch(() => props.resetKey, () => { jsonDirty.value = false; jsonDraft.value = JSON.stringify(model.value, null, 2); });
function applyJson(): void {
  try { const value = parseGrant(JSON.parse(jsonDraft.value)); jsonDirty.value = false; invalid.value = false; model.value = value; emit('applied'); }
  catch { invalid.value = true; }
}

</script>

<template>
  <div class="grant-editor__advanced">
    <section
      class="grant-editor__panel"
    >
      <h3>{{ t('collaboration.futurePermissions') }}</h3>
      <p>{{ t('collaboration.futureHint') }}</p>
      <BaseSwitch
        v-for="capability in capabilities.filter((c) => !current.some((v) => v === c) && !effectCaps.some((v) => v === c))"
        :key="capability"
        :model-value="model.capabilities.includes(capability)"
        :disabled="disabled || requiredByDelegation(capability)"
        @update:model-value="setCapability(capability, $event)"
      >
        {{ capability }}
      </BaseSwitch>
    </section>      <section
      class="grant-editor__panel"
    >
      <h3>{{ t('collaboration.extraGrant') }}</h3>
      <BaseTextArea
        v-model="jsonDraft"
        :disabled="disabled"
        :aria-label="t('collaboration.extraGrant')"
        :rows="8"
        maxlength="262144"
        spellcheck="false"
        @update:model-value="jsonDirty = true"
      />
      <BaseButton
        :disabled="disabled"
        @click="applyJson"
      >
        {{ t('collaboration.applyJson') }}
      </BaseButton>
      <p
        v-if="invalid"
        role="alert"
      >
        {{ t('collaboration.invalidGrant') }}
      </p>
    </section>      <section
      class="grant-editor__panel"
    >
      <h3>{{ t('collaboration.grantSummary') }}</h3>
      <BaseJsonView
        :value="model"
        max-height="300px"
      />
    </section>
  </div>
</template>
