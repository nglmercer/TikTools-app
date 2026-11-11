<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import type { ProcessorStatusEntry } from '../../../../shared/messages.ts';
import { processorMetricRows, processorPreviewEvent, processorStatusTone } from '@/entities/processor/lib/processors.ts';
import { useProcessorsStore } from '@/entities/processor/model/processors-store.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconFlask from '@/shared/ui/icon/icons/IconFlask.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import { processorKey } from '../lib/processor-rows.ts';

const props = defineProps<{
  /** Key of the open processor, as `processorKey` builds it. */
  selected: string;
  plugins: readonly PluginStatus[];
}>();

const emit = defineEmits<{ back: [] }>();

const { t } = useI18n();
const store = useProcessorsStore();
const { processors, test, testing } = storeToRefs(store);

const current = computed(() => processors.value.find((entry) => processorKey(entry) === props.selected) ?? null);
const currentTest = computed(() => {
  const entry = current.value;
  const outcome = test.value;
  return entry && outcome && outcome.pluginId === entry.pluginId && outcome.processorId === entry.processorId ? outcome : null;
});
const facts = computed(() => {
  const entry = current.value;
  if (!entry) return [];
  const metrics = processorMetricRows(entry.metrics).map((metric) => ({
    key: metric.key,
    label: t(`plugins.processors.metrics.${metric.key}`),
    value: metric.value,
    mono: false,
  }));
  return [
    { key: 'plugin', label: t('plugins.processors.columns.plugin'), value: pluginName(entry.pluginId), mono: false },
    { key: 'events', label: t('plugins.processors.columns.events'), value: eventsLabel(entry), mono: true },
    { key: 'timeout', label: t('plugins.processors.timeout'), value: t('plugins.processors.milliseconds', { value: entry.timeoutMs }), mono: false },
    { key: 'priority', label: t('plugins.processors.priority'), value: String(entry.priority), mono: false },
    ...metrics,
  ];
});

function pluginName(pluginId: string): string {
  const plugin = props.plugins.find((entry) => entry.descriptor.id === pluginId);
  return plugin ? localizedText(plugin.descriptor.name) : pluginId;
}

function eventsLabel(entry: ProcessorStatusEntry): string {
  return entry.eventTypes.length ? entry.eventTypes.join(', ') : t('plugins.processors.allEvents');
}

function runTest(entry: ProcessorStatusEntry): void {
  void store.runTest(entry.pluginId, entry.processorId, processorPreviewEvent(entry));
}
</script>

<template>
  <BasePanel
    v-if="current"
    class="processor-details"
    :title="current.processorId"
    flush
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        size="sm"
        @click="emit('back')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
    <template #status>
      <BaseStatus
        :label="t(`plugins.processors.status.${current.status}`)"
        :tone="processorStatusTone(current.status) === 'ok' ? 'success' : processorStatusTone(current.status) === 'error' ? 'error' : 'neutral'"
      />
    </template>
    <template #actions>
      <BaseButton
        variant="primary"
        :disabled="testing !== null"
        @click="runTest(current)"
      >
        <template #icon>
          <IconFlask />
        </template>
        {{ testing === selected ? t('plugins.processors.testing') : t('plugins.processors.test') }}
      </BaseButton>
    </template>

    <div class="processor-details__body">
      <BaseSection :title="t('plugins.processors.columns.processor')">
        <dl class="processor-details__facts">
          <div
            v-for="fact in facts"
            :key="fact.key"
            class="processor-details__fact"
          >
            <dt>{{ fact.label }}</dt>
            <dd :class="{ 'processor-details__mono': fact.mono }">
              {{ fact.value }}
            </dd>
          </div>
        </dl>
      </BaseSection>
      <section
        v-if="current.metrics.lastError"
        class="processor-details__alert processor-details__alert--failed"
      >
        <h3>{{ t('plugins.processors.lastError') }}</h3>
        <p>{{ current.metrics.lastError }}</p>
      </section>
      <section
        v-if="currentTest"
        class="processor-details__alert"
        :class="{ 'processor-details__alert--failed': !currentTest.ok }"
        role="status"
      >
        <h3>
          {{
            currentTest.ok
              ? t('plugins.processors.testOk', { ms: currentTest.durationMs })
              : t('plugins.processors.testFailed', { ms: currentTest.durationMs })
          }}
        </h3>
        <p v-if="currentTest.error">
          {{ currentTest.error }}
        </p>
        <pre class="processor-details__result">{{ JSON.stringify(currentTest.result, null, 2) }}</pre>
      </section>
    </div>
  </BasePanel>
</template>

<style scoped>
.processor-details {
  flex: 1;
}

.processor-details__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
}

.processor-details__facts {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: var(--space-3) var(--space-5);
  margin: 0;
}

.processor-details__fact {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.processor-details__fact dt {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.processor-details__fact dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.processor-details__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.processor-details__alert {
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.processor-details__alert--failed {
  border-color: var(--color-danger);
}

.processor-details__alert h3 {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.processor-details__alert p {
  margin: 0 0 var(--space-2);
  color: var(--color-danger-hover);
  overflow-wrap: anywhere;
}

.processor-details__result {
  max-height: 280px;
  margin: 0;
  padding: var(--space-3);
  overflow: auto;
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  white-space: pre-wrap;
}
</style>
