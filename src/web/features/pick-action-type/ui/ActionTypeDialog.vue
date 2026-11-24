<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import { matchesQuery } from '@/entities/automation/lib/automation-records.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{
  plugins: readonly PluginStatus[];
  actionTypes: readonly ActionTypeDefinition[];
}>();

const emit = defineEmits<{ pick: [type: ActionTypeDefinition]; openPlugins: [] }>();
const open = defineModel<boolean>('open', { default: false });

const { t } = useI18n();
const query = ref('');

function matches(type: ActionTypeDefinition): boolean {
  const pluginId = type.source.kind === 'plugin' ? type.source.pluginId : '';
  return matchesQuery(
    query.value,
    type.id,
    type.tag ?? '',
    type.category ?? '',
    localizedText(type.title),
    localizedText(type.description),
    pluginId,
  );
}

const groups = computed(() => {
  const builtin = props.actionTypes.filter((type) => type.source.kind === 'builtin' && matches(type));
  const fromPlugins = props.plugins
    .filter((plugin) => plugin.installed && plugin.enabled)
    .map((plugin) => ({
      id: plugin.descriptor.id,
      title: localizedText(plugin.descriptor.name),
      note: localizedText(plugin.descriptor.dependency),
      types: props.actionTypes.filter(
        (type) => type.source.kind === 'plugin' && type.source.pluginId === plugin.descriptor.id && matches(type),
      ),
    }))
    .filter((group) => group.types.length > 0);
  return [
    ...(builtin.length ? [{ id: 'builtin', title: t('automations.picker.builtin'), note: t('automations.picker.builtinNote'), types: builtin }] : []),
    ...fromPlugins,
  ];
});

watch(open, (value) => {
  if (value) query.value = '';
});

function pick(type: ActionTypeDefinition): void {
  open.value = false;
  emit('pick', type);
}

function browsePlugins(): void {
  open.value = false;
  emit('openPlugins');
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t('automations.picker.title')"
    :width="680"
  >
    <p class="type-picker__lead">
      {{ t('automations.picker.lead') }}
    </p>
    <BaseTextField
      v-model="query"
      type="search"
      :placeholder="t('automations.picker.search')"
      :aria-label="t('automations.picker.search')"
    >
      <template #prefix>
        <IconSearch />
      </template>
    </BaseTextField>
    <div class="type-picker__groups">
      <p
        v-if="!groups.length"
        class="type-picker__empty"
      >
        {{ t('automations.picker.empty') }}
      </p>
      <section
        v-for="group in groups"
        :key="group.id"
        class="type-picker__group"
      >
        <h3 class="type-picker__heading">
          {{ group.title }}
          <span class="type-picker__note">{{ group.note }}</span>
        </h3>
        <div class="type-picker__grid">
          <button
            v-for="type in group.types"
            :key="type.id"
            class="type-picker__card"
            type="button"
            @click="pick(type)"
          >
            <span class="type-picker__title">
              {{ localizedText(type.title) }}
              <span class="type-picker__tag">{{ type.tag }}</span>
            </span>
            <span class="type-picker__description">{{ localizedText(type.description) }}</span>
          </button>
        </div>
      </section>
      <div class="type-picker__missing">
        <span>
          <strong>{{ t('automations.picker.missingTitle') }}</strong>
          {{ t('automations.picker.missingLead') }}
        </span>
        <BaseButton
          size="sm"
          @click="browsePlugins"
        >
          {{ t('automations.picker.browsePlugins') }}
        </BaseButton>
      </div>
    </div>
  </BaseDialog>
</template>

<style scoped>
.type-picker__lead {
  margin: 0 0 var(--space-4);
  color: var(--color-text-muted);
}

.type-picker__groups {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  max-height: min(60vh, 520px);
  max-height: min(60dvh, 520px);
  margin-top: var(--space-4);
  overflow-y: auto;
}

.type-picker__empty {
  margin: 0;
  color: var(--color-text-subtle);
}

.type-picker__heading {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  margin: 0 0 var(--space-3);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.type-picker__note {
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-regular);
}

.type-picker__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--space-2);
}

.type-picker__card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-start;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  text-align: left;
  cursor: pointer;
}

.type-picker__card:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-control-hover);
}

.type-picker__card:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.type-picker__title {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-weight: var(--font-weight-semibold);
}

.type-picker__tag {
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-regular);
}

.type-picker__description {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.type-picker__missing {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) dashed var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.type-picker__missing > span {
  flex: 1;
}

.type-picker__missing strong {
  color: var(--color-text);
}
</style>
