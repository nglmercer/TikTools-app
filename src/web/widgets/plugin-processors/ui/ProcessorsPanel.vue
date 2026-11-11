<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import { processorStatusTone } from '@/entities/processor/lib/processors.ts';
import { useProcessorsStore } from '@/entities/processor/model/processors-store.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import { filterProcessors, processorKey } from '../lib/processor-rows.ts';

const props = defineProps<{
  plugins: readonly PluginStatus[];
  query: string;
}>();

const emit = defineEmits<{ open: [key: string] }>();

const { t } = useI18n();
const store = useProcessorsStore();
const { processors, error } = storeToRefs(store);

const rows = computed(() =>
  filterProcessors(processors.value, props.query).map((entry) => ({
    key: processorKey(entry),
    id: entry.processorId,
    plugin: pluginName(entry.pluginId),
    status: entry.status,
    tone: processorStatusTone(entry.status),
  })),
);

function pluginName(pluginId: string): string {
  const plugin = props.plugins.find((entry) => entry.descriptor.id === pluginId);
  return plugin ? localizedText(plugin.descriptor.name) : pluginId;
}

onMounted(() => void store.refresh());
</script>

<template>
  <div class="processors">
    <p
      v-if="error"
      class="processors__error"
      role="alert"
    >
      {{ error }}
    </p>
    <ul
      v-if="rows.length"
      class="processors__list"
      :aria-label="t('plugins.views.processors')"
    >
      <li
        v-for="row in rows"
        :key="row.key"
        class="processors__row"
      >
        <button
          class="processors__open"
          type="button"
          @click="emit('open', row.key)"
        >
          <span class="processors__name">{{ row.id }}</span>
          <span class="processors__sub">{{ row.plugin }}</span>
        </button>
        <!-- Only a processor that is not ready needs a status; a healthy one says nothing. -->
        <span
          v-if="row.tone !== 'ok'"
          class="processors__status"
          :class="`processors__status--${row.tone}`"
        >{{ t(`plugins.processors.status.${row.status}`) }}</span>
        <IconChevronRight class="processors__chevron" />
      </li>
    </ul>
    <BaseEmptyState
      v-else-if="query.trim()"
      :title="t('plugins.processors.noMatches', { query: query.trim() })"
    />
    <BaseEmptyState
      v-else
      :title="t('plugins.processors.empty')"
      :description="t('plugins.processors.emptyHint')"
    >
      <template #icon>
        <IconFlow :size="28" />
      </template>
    </BaseEmptyState>
  </div>
</template>

<style scoped>
.processors__error {
  margin: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.processors__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.processors__row {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: calc(var(--control-height-lg) + var(--space-3));
  padding: 0 var(--space-4);
}

.processors__row + .processors__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.processors__row:hover {
  background: var(--color-bg-hover);
}

.processors__open {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
  padding: var(--space-3) 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.processors__open::after {
  position: absolute;
  inset: 0;
  content: '';
}

.processors__open:focus-visible {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: calc(var(--border-width) * -1);
}

.processors__name {
  overflow: hidden;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.processors__sub {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.processors__status {
  flex: none;
  color: var(--color-warning);
  font-size: var(--font-size-sm);
}

.processors__status--error {
  color: var(--color-danger-hover);
}

.processors__chevron {
  flex: none;
  color: var(--color-text-muted);
}
</style>
