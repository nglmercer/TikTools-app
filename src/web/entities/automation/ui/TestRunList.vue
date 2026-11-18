<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BehaviorRun } from '../../../../automation/behavior/types.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCode from '@/shared/ui/icon/icons/IconCode.vue';
import RunEventDataView from './RunEventDataView.vue';

defineProps<{
  runs: readonly BehaviorRun[];
}>();

const { t } = useI18n();
const selected = ref(-1);
function toggle(index: number): void {
  selected.value = selected.value === index ? -1 : index;
}
</script>

<template>
  <section
    class="test-runs"
    :aria-label="t('automations.test.title')"
  >
    <h3 class="test-runs__title">
      {{ t('automations.test.title') }}
    </h3>
    <p
      v-if="!runs.length"
      class="test-runs__empty"
    >
      {{ t('automations.test.empty') }}
    </p>
    <ul
      v-else
      class="test-runs__list"
      role="log"
    >
      <li
        v-for="(run, index) in runs"
        :key="run.id"
        class="test-runs__item"
        :class="`test-runs__item--${run.status}`"
      >
        <span class="test-runs__head">
          <span
            class="test-runs__dot"
            aria-hidden="true"
          />
          <span class="test-runs__name">{{ run.actionName }}</span>
          <span
            v-if="run.eventSource"
            class="test-runs__source"
          >{{ t(`automations.test.sources.${run.eventSource}`) }}</span>
          <span class="test-runs__duration">{{ t('automations.test.duration', { ms: run.durationMs }) }}</span>
          <BaseIconButton
            v-if="run.eventData !== undefined"
            :label="selected === index ? t('automations.eventData.hide') : t('automations.eventData.show')"
            size="sm"
            :aria-expanded="selected === index"
            @click="toggle(index)"
          >
            <IconCode />
          </BaseIconButton>
        </span>
        <span class="test-runs__summary">{{ run.error ?? run.summary }}</span>
        <pre
          v-if="run.logs.length"
          class="test-runs__logs"
        >{{ run.logs.join('\n') }}</pre>
        <RunEventDataView
          v-if="selected === index"
          :runs="runs"
          :index="index"
          @navigate="selected = $event"
          @close="selected = -1"
        />
      </li>
    </ul>
  </section>
</template>

<style scoped>
.test-runs {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.test-runs__title {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.test-runs__empty {
  margin: 0;
  color: var(--color-text-subtle);
}

.test-runs__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.test-runs__item {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  border-left: 3px solid var(--color-success);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
}

.test-runs__item--error {
  border-left-color: var(--color-danger);
}

.test-runs__head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.test-runs__dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-success);
}

.test-runs__item--error .test-runs__dot {
  background: var(--color-danger);
}

.test-runs__name {
  flex: 1;
  font-weight: var(--font-weight-semibold);
}

.test-runs__source,
.test-runs__duration {
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.test-runs__summary {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  overflow-wrap: anywhere;
}

.test-runs__item--error .test-runs__summary {
  color: var(--color-danger-hover);
}

.test-runs__logs {
  max-height: 160px;
  margin: 0;
  overflow: auto;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  white-space: pre-wrap;
}
</style>
