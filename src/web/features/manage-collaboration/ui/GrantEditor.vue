<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { capabilities, viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';
import { cloneGrant } from '../model/clone-grant.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import GrantPermissions from './GrantPermissions.vue';
import GrantDelegation from './GrantDelegation.vue';
import GrantEffects from './GrantEffects.vue';
import GrantResources from './GrantResources.vue';
import AdvancedGrantEditor from './AdvancedGrantEditor.vue';

const props = defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const section = ref('permissions');
const jsonReset = ref(0);
const scopeReset = ref(0);
const sections = computed(() => ['permissions', 'delegation', 'effects', 'resources', 'advanced'].map(value => ({ value, label: t(`collaboration.editorSections.${value}`) })));
function setAll(enabled: boolean): void {
  if (props.disabled) return;
  const value = cloneGrant(model.value);
  value.capabilities = enabled ? [...capabilities] : [];
  if (!enabled) delete value.automationDelegation;
  model.value = value;
  jsonReset.value += 1;
}
function preset(kind: 'viewer' | 'workflow' | 'agent'): void {
  const value = viewerGrant();
  if (kind !== 'viewer') { value.capabilities.push('workflows.read'); value.resources.workflow = []; }
  if (kind === 'agent') value.capabilities.push('commands.read');
  model.value = value;
  jsonReset.value += 1;
  scopeReset.value += 1;
}
</script>

<template>
  <div class="grant-editor">
    <div class="grant-editor__actions">
      <BaseButton
        :disabled="disabled"
        @click="preset('viewer')"
      >
        {{ t('collaboration.presetViewer') }}
      </BaseButton>
      <BaseButton
        :disabled="disabled"
        @click="preset('workflow')"
      >
        {{ t('collaboration.presetWorkflow') }}
      </BaseButton>
      <BaseButton
        :disabled="disabled"
        @click="preset('agent')"
      >
        {{ t('collaboration.presetAgent') }}
      </BaseButton>
    </div>
    <div
      class="grant-editor__bulk"
      role="group"
      :aria-label="t('collaboration.bulkLabel')"
    >
      <BaseButton
        :disabled="disabled"
        @click="setAll(true)"
      >
        {{ t('collaboration.enableAll') }}
      </BaseButton>
      <BaseButton
        :disabled="disabled"
        @click="setAll(false)"
      >
        {{ t('collaboration.disableAll') }}
      </BaseButton>
      <span>{{ t('collaboration.bulkHint') }}</span>
    </div>
    <BaseTabs
      v-model="section"
      :tabs="sections"
      :label="t('collaboration.grantSummary')"
      class="grant-editor__tabs"
    >
      <GrantPermissions
        v-show="section === 'permissions'"
        v-model="model"
        :disabled="disabled"
      />
      <AdvancedGrantEditor
        v-show="section === 'advanced'"
        v-model="model"
        :disabled="disabled"
        :reset-key="jsonReset"
        @applied="scopeReset += 1"
      />
      <GrantDelegation
        v-show="section === 'delegation'"
        v-model="model"
        :disabled="disabled"
      />
      <GrantEffects
        v-show="section === 'effects'"
        v-model="model"
        :disabled="disabled"
      />
      <GrantResources
        v-show="section === 'resources'"
        v-model="model"
        :disabled="disabled"
        :reset-key="scopeReset"
      />
    </BaseTabs>
  </div>
</template>

<style src="./grant-editor.css"></style>
