<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BehaviorRun } from '../../../../automation/behavior/types.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';

/**
 * Inline inspector for the `event.data` of a run. It stays inside the list
 * that owns the runs (no dialog), navigates between runs with the same
 * controls for every trigger, and delegates rendering/bounds to BaseJsonView.
 */
const props = withDefaults(defineProps<{
  runs: readonly BehaviorRun[];
  index: number;
  /** The list is hidden while inspecting: label the exit as "back to list". */
  back?: boolean;
}>(), { back: false });
const emit = defineEmits<{ navigate: [index: number]; close: [] }>();
const { t } = useI18n();

const run = computed(() => props.runs[props.index]);
const hasData = computed(() => run.value?.eventData !== undefined && run.value.eventData !== null);
const sourceLabel = computed(() => {
  const source = run.value?.eventSource;
  return source ? String(t(`automations.test.sources.${source}`)) : '';
});

// Empty arrays/objects/null are noise in a payload preview (e.g. `emotes: []`).
const EMPTY = Symbol('empty');
function compact(value: unknown): unknown {
  if (Array.isArray(value)) {
    const items = value.map(compact).filter((item) => item !== EMPTY);
    return items.length ? items : EMPTY;
  }
  if (value && typeof value === 'object') {
    const record: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value)) {
      const next = compact(item);
      if (next !== EMPTY) record[key] = next;
    }
    return Object.keys(record).length ? record : EMPTY;
  }
  return value === null || value === '' ? EMPTY : value;
}
const viewValue = computed(() => {
  const value = compact(run.value?.eventData);
  return value === EMPTY ? null : value;
});
</script>

<template>
  <div class="run-event-data">
    <div
      class="run-event-data__nav"
      tabindex="0"
      @keydown.left.prevent="index > 0 && emit('navigate', index - 1)"
      @keydown.right.prevent="index < runs.length - 1 && emit('navigate', index + 1)"
    >
      <span
        v-if="sourceLabel"
        class="run-event-data__source"
      >{{ sourceLabel }}</span>
      <BaseIconButton
        :label="t('automations.eventData.previous')"
        size="sm"
        :disabled="index <= 0"
        @click="emit('navigate', index - 1)"
      >
        <IconChevronLeft />
      </BaseIconButton>
      <span class="run-event-data__position">{{ index + 1 }} / {{ runs.length }}</span>
      <BaseIconButton
        :label="t('automations.eventData.next')"
        size="sm"
        :disabled="index >= runs.length - 1"
        @click="emit('navigate', index + 1)"
      >
        <IconChevronRight />
      </BaseIconButton>
      <BaseIconButton
        :label="t(back ? 'automations.eventData.back' : 'automations.eventData.hide')"
        size="sm"
        @click="emit('close')"
      >
        <IconChevronLeft v-if="back" />
        <IconClose v-else />
      </BaseIconButton>
    </div>
    <p
      v-if="!hasData"
      class="run-event-data__empty"
    >
      {{ t('automations.eventData.empty') }}
    </p>
    <BaseJsonView
      v-else
      :value="viewValue"
      :label="t('automations.eventData.label')"
      max-height="240px"
      line-numbers
      switchable
      copyable
      :max-lines="500"
      :max-chars="200000"
    />
  </div>
</template>

<style scoped>
.run-event-data {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-top: var(--space-2);
  padding-top: var(--space-2);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.run-event-data__nav {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.run-event-data__source {
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}

.run-event-data__position {
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
}

.run-event-data__empty {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}
</style>
