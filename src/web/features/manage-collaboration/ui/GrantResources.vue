<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { ref, watch } from 'vue';
import { resourceKinds, type ResourceKind } from '@/entities/collaboration/api/types.ts';
import { splitIds } from '@/entities/collaboration/lib/grants.ts';
import { cloneGrant } from '../model/clone-grant.ts';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
const props = defineProps<{ disabled?: boolean; resetKey: number }>();
const model = defineModel<Grant>({ required: true });
const { t } = useI18n();
type ScopeMode = 'all' | 'selected' | 'none';
const modes = ref<Partial<Record<ResourceKind, ScopeMode>>>({});
watch([model, () => props.resetKey], ([value, key], previous) => {
  if (key !== previous?.[1]) modes.value = {};
  for (const kind of resourceKinds) {
    const ids = value.resources[kind];
    modes.value[kind] = ids === undefined ? 'all' : ids.length || modes.value[kind] === 'selected' ? 'selected' : 'none';
  }
}, { immediate: true, deep: true });
function setMode(kind: ResourceKind, mode: 'all' | 'selected' | 'none'): void {
  const value = cloneGrant(model.value); modes.value[kind] = mode;
  if (mode === 'all') delete value.resources[kind];
  else value.resources[kind] = mode === 'none' ? [] : value.resources[kind] ?? [];
  model.value = value;
}
function setIds(kind: ResourceKind, text: string): void { const value = cloneGrant(model.value); value.resources[kind] = splitIds(text); model.value = value; }

</script>

<template>
  <section
    class="grant-editor__panel"
  >
    <div class="grant-editor__group">
      <h4 class="grant-editor__group-title">
        <span>{{ t('collaboration.scopes') }}</span>
      </h4>
    </div>
    <div class="grant-editor__scopes">
      <div
        v-for="kind in resourceKinds"
        :key="kind"
        class="grant-editor__scope"
      >
        <h4>{{ t(`collaboration.scopeKinds.${kind}`) }}</h4>
        <BaseSegmented
          size="sm"
          :model-value="modes[kind] ?? 'all'"
          :disabled="disabled"
          :aria-label="t(`collaboration.scopeKinds.${kind}`)"
          :options="(['all', 'selected', 'none'] as const).map(value => ({ value, label: t(`collaboration.scopeModes.${value}`) }))"
          @update:model-value="setMode(kind, $event)"
        />
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
        <p
          v-if="kind === 'automation'"
          class="grant-editor__hint"
        >
          {{ t('collaboration.automationScopeHint') }}
        </p>
      </div>
    </div>
  </section>
</template>
