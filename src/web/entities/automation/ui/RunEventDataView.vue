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
</script>

<template>
  <div class="run-event-data">
    <div class="run-event-data__nav">
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
      :value="run?.eventData"
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
