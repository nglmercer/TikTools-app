<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { capabilities, resourceKinds, viewerGrant, type AutomationDelegation, type Capability, type EffectGrants, type Grant, type ResourceKind } from '@/entities/collaboration/api/types.ts';
import { parseGrant, splitIds } from '@/entities/collaboration/lib/grants.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import EffectGrantEditor from './EffectGrantEditor.vue';

defineProps<{ disabled?: boolean }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
const current = ['live.read', 'workflows.read', 'workflows.toggle', 'workflows.edit', 'automation.read', 'automation.toggle', 'automation.edit', 'points.read', 'points.adjust', 'commands.read'] as const;
const labels = { 'live.read': 'capLive', 'workflows.read': 'capWorkflow', 'workflows.toggle': 'capWorkflowDisable', 'workflows.edit': 'capWorkflowDelete', 'automation.read': 'capAutomation', 'automation.toggle': 'capEventDisable', 'automation.edit': 'capEventDelete', 'points.read': 'capPoints', 'points.adjust': 'capPointsAdjust', 'commands.read': 'capCommands' } as const;
const effectCaps = ['http.request', 'audio.play', 'events.emit', 'scripts.execute'] as const;
const effectLabels = { 'http.request': 'capHttp', 'audio.play': 'capAudio', 'events.emit': 'capEmit', 'scripts.execute': 'capScript' } as const;
type DelegatedEffect = AutomationDelegation['effects'][number];
const delegatedEffects: DelegatedEffect[] = ['log', 'delay', 'points', 'http', 'audio', 'emit', 'script'];
const effectCapabilities: Partial<Record<DelegatedEffect, Capability[]>> = { points: ['points.read', 'points.adjust'], http: ['http.request'], audio: ['audio.play'], emit: ['events.emit'], script: ['scripts.execute', 'live.read'] };
const requiredAutomation: Capability[] = ['automation.read', 'automation.edit', 'automation.toggle'];
function requiredByDelegation(capability: Capability): boolean {
  const delegation = model.value.automationDelegation;
  return !!delegation && model.value.capabilities.includes(capability) && (requiredAutomation.includes(capability) || delegation.effects.some((effect) => effectCapabilities[effect]?.includes(capability)));
}
function setDelegatedEffect(effect: DelegatedEffect, enabled: boolean): void {
  const value = copy();
  const effects = new Set(value.automationDelegation?.effects ?? []);
  if (enabled) effects.add(effect); else effects.delete(effect);
  if (!effects.size) delete value.automationDelegation;
  else {
    value.automationDelegation = { ...value.automationDelegation, effects: [...effects], allowDynamicTargets: value.automationDelegation?.allowDynamicTargets ?? false };
    const caps = new Set(value.capabilities);
    requiredAutomation.forEach((capability) => caps.add(capability));
    if (enabled) effectCapabilities[effect]?.forEach((capability) => caps.add(capability));
    value.capabilities = [...caps];
  }
  model.value = value;
}
function setOwnerEdits(enabled: boolean): void {
  const value = copy();
  if (value.automationDelegation) {
    if (enabled) value.automationDelegation.allowOwnerEdits = true;
    else { delete value.automationDelegation.allowOwnerEdits; delete value.automationDelegation.allowOwnerSource; }
  }
  model.value = value;
}
function setOwnerSource(enabled: boolean): void {
  const value = copy();
  if (value.automationDelegation && value.automationDelegation.allowOwnerEdits) {
    if (enabled) value.automationDelegation.allowOwnerSource = true;
    else delete value.automationDelegation.allowOwnerSource;
  }
  model.value = value;
}
function setDynamicTargets(enabled: boolean): void {
  const value = copy();
  if (value.automationDelegation) value.automationDelegation.allowDynamicTargets = enabled;
  model.value = value;
}
const modes = ref<Partial<Record<ResourceKind, 'all' | 'selected' | 'none'>>>({});
const jsonDraft = ref('');
const jsonDirty = ref(false);
const invalid = ref(false);
watch(model, (value) => {
  for (const kind of resourceKinds) {
    const ids = value.resources[kind];
    modes.value[kind] = ids === undefined ? 'all' : ids.length || modes.value[kind] === 'selected' ? 'selected' : 'none';
  }
  if (!jsonDirty.value) jsonDraft.value = JSON.stringify(value, null, 2);
}, { immediate: true, deep: true });
const copy = () => JSON.parse(JSON.stringify(model.value)) as Grant;
function setCapability(capability: Capability, allowed: boolean): void {
  const value = copy(); const caps = new Set(value.capabilities);
  if (allowed) caps.add(capability); else caps.delete(capability);
  value.capabilities = [...caps]; model.value = value;
}
function setMode(kind: ResourceKind, mode: 'all' | 'selected' | 'none'): void {
  const value = copy(); modes.value[kind] = mode;
  if (mode === 'all') delete value.resources[kind];
  else value.resources[kind] = mode === 'none' ? [] : value.resources[kind] ?? [];
  model.value = value;
}
function setIds(kind: ResourceKind, text: string): void { const value = copy(); value.resources[kind] = splitIds(text); model.value = value; }
function setPointsLimit(event: Event): void {
  const input = event.target as HTMLInputElement;
  const value = copy();
  value.maxPointsDelta = input.validity.badInput ? 0 : input.value === '' ? null : Number.isFinite(input.valueAsNumber) ? input.valueAsNumber : 0;
  model.value = value;
}
function setEffects(effects: EffectGrants | undefined): void { const value = copy(); if (effects) value.effects = effects; else delete value.effects; model.value = value; }
function preset(kind: 'viewer' | 'workflow' | 'agent'): void {
  const value = viewerGrant();
  if (kind !== 'viewer') { value.capabilities.push('workflows.read'); value.resources.workflow = []; }
  if (kind === 'agent') value.capabilities.push('commands.read');
  jsonDirty.value = false; modes.value = {}; model.value = value;
}
function applyJson(): void {
  try { const value = parseGrant(JSON.parse(jsonDraft.value)); jsonDirty.value = false; invalid.value = false; modes.value = {}; model.value = value; }
  catch { invalid.value = true; }
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
    <BaseCheckbox
      v-for="capability in current"
      :key="capability"
      :model-value="model.capabilities.includes(capability)"
      :disabled="disabled || requiredByDelegation(capability)"
      @update:model-value="setCapability(capability, $event)"
    >
      {{ t(`collaboration.${labels[capability]}`) }}
    </BaseCheckbox>
    <label v-if="model.capabilities.includes('points.adjust')">
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
    <details>
      <summary>{{ t('collaboration.futurePermissions') }}</summary>
      <p>{{ t('collaboration.futureHint') }}</p>
      <BaseCheckbox
        v-for="capability in capabilities.filter((c) => !current.some((v) => v === c) && !effectCaps.some((v) => v === c))"
        :key="capability"
        :model-value="model.capabilities.includes(capability)"
        :disabled="disabled || requiredByDelegation(capability)"
        @update:model-value="setCapability(capability, $event)"
      >
        {{ capability }}
      </BaseCheckbox>
    </details>
    <details>
      <summary>{{ t('collaboration.delegationTitle') }}</summary>
      <p>{{ t('collaboration.delegationHint') }}</p>
      <p>{{ t('collaboration.delegationReadHint') }}</p>
      <BaseCheckbox
        v-for="effect in delegatedEffects"
        :key="effect"
        :model-value="model.automationDelegation?.effects.includes(effect) ?? false"
        :disabled="disabled"
        @update:model-value="setDelegatedEffect(effect, $event)"
      >
        {{ t(`collaboration.delegationEffects.${effect}`) }}
      </BaseCheckbox>
      <BaseCheckbox
        :model-value="model.automationDelegation?.allowOwnerEdits ?? false"
        :disabled="disabled || !model.automationDelegation"
        @update:model-value="setOwnerEdits"
      >
        {{ t('collaboration.delegationOwnerEdits') }}
      </BaseCheckbox>
      <p>{{ t('collaboration.delegationOwnerEditsHint') }}</p>
      <BaseCheckbox
        :model-value="model.automationDelegation?.allowOwnerSource ?? false"
        :disabled="disabled || !model.automationDelegation?.allowOwnerEdits"
        @update:model-value="setOwnerSource"
      >
        {{ t('collaboration.delegationOwnerSource') }}
      </BaseCheckbox>
      <p>{{ t('collaboration.delegationOwnerSourceHint') }}</p>
      <BaseCheckbox
        :model-value="model.automationDelegation?.allowDynamicTargets ?? false"
        :disabled="disabled || !model.automationDelegation"
        @update:model-value="setDynamicTargets"
      >
        {{ t('collaboration.delegationDynamic') }}
      </BaseCheckbox>
    </details>
    <details>
      <summary>{{ t('collaboration.effectRules') }}</summary>
      <BaseCheckbox
        v-for="capability in effectCaps"
        :key="capability"
        :model-value="model.capabilities.includes(capability)"
        :disabled="disabled || requiredByDelegation(capability)"
        @update:model-value="setCapability(capability, $event)"
      >
        {{ t(`collaboration.${effectLabels[capability]}`) }}
      </BaseCheckbox>
      <EffectGrantEditor
        :model-value="model.effects"
        :disabled="disabled"
        @update:model-value="setEffects"
      />
    </details>
    <details>
      <summary>{{ t('collaboration.scopes') }}</summary>
      <div
        v-for="kind in resourceKinds"
        :key="kind"
        class="grant-editor__scope"
      >
        <label>{{ t(`collaboration.scopeKinds.${kind}`) }}<select
          :value="modes[kind]"
          :disabled="disabled"
          @change="setMode(kind, ($event.target as HTMLSelectElement).value as 'all' | 'selected' | 'none')"
        ><option value="all">{{ t('collaboration.all') }}</option><option value="selected">{{ t('collaboration.selected') }}</option><option value="none">{{ t('collaboration.none') }}</option></select></label>
        <BaseTextArea
          v-if="modes[kind] === 'selected'"
          :model-value="model.resources[kind]?.join('\n') ?? ''"
          :aria-label="`${t(`collaboration.scopeKinds.${kind}`)} — ${t('collaboration.ids')}`"
          :placeholder="t(kind === 'automation' ? 'collaboration.automationIds' : 'collaboration.ids')"
          :disabled="disabled"
          :rows="3"
          maxlength="65536"
          @update:model-value="setIds(kind, $event)"
        />
        <p v-if="kind === 'automation'">
          {{ t('collaboration.automationScopeHint') }}
        </p>
      </div>
    </details>
    <details>
      <summary>{{ t('collaboration.extraGrant') }}</summary>
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
    </details>
    <details><summary>{{ t('collaboration.grantSummary') }}</summary><pre>{{ JSON.stringify(model, null, 2) }}</pre></details>
  </div>
</template>

<style scoped>
.grant-editor { display: flex; flex-direction: column; gap: var(--space-3); }
.grant-editor__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.grant-editor__scope { margin: var(--space-3) 0; }
.grant-editor label { display: grid; gap: var(--space-2); }
.grant-editor select, .grant-editor input { padding: var(--space-3); color: var(--color-text); background: var(--color-bg-field); border: 1px solid var(--color-border); }
pre { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 300px; overflow: auto; } summary { cursor: pointer; }
</style>
