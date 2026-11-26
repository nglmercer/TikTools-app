<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { analyticsRangeKeys, isoDateToDay, type AnalyticsRangeKey } from '@/entities/analytics/lib/analytics-range.ts';
import BaseDateRange from '@/shared/ui/date-range/BaseDateRange.vue';
import BaseDateText from '@/shared/ui/date-text/BaseDateText.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

const range = defineModel<AnalyticsRangeKey>('range', { required: true });
const start = defineModel<string>('start', { required: true });
const end = defineModel<string>('end', { required: true });

const { t } = useI18n();
const options = computed(() => analyticsRangeKeys.map((key) => ({ value: key, label: t(`analytics.ranges.${key}`) })));

// The custom inputs live behind a compact popover trigger: the full-width
// stacked fields consumed the whole mobile header for a one-time choice.
const open = ref(false);
const root = ref<HTMLElement | null>(null);
const startDay = computed(() => isoDateToDay(start.value) ?? 0);
const endDay = computed(() => isoDateToDay(end.value) ?? 0);

function onDocumentClick(event: MouseEvent): void {
  if (root.value && event.target instanceof Node && !root.value.contains(event.target)) open.value = false;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') open.value = false;
}

watch(open, (active) => {
  if (active) {
    document.addEventListener('mousedown', onDocumentClick);
    document.addEventListener('keydown', onKeydown);
  } else {
    document.removeEventListener('mousedown', onDocumentClick);
    document.removeEventListener('keydown', onKeydown);
  }
});
onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocumentClick);
  document.removeEventListener('keydown', onKeydown);
});

// Switching to a preset closes the popover; picking custom opens it so the
// dates are one click away instead of buried.
watch(range, (value) => {
  open.value = value === 'custom';
});
</script>

<template>
  <div
    ref="root"
    class="range-bar"
  >
    <BaseSegmented
      v-model="range"
      :options="options"
      :aria-label="t('analytics.rangeLabel')"
    />
    <div
      v-if="range === 'custom'"
      class="range-bar__custom"
    >
      <button
        type="button"
        class="range-bar__trigger"
        :aria-expanded="open"
        aria-haspopup="dialog"
        :aria-label="t('analytics.ranges.custom')"
        @click="open = !open"
      >
        <BaseDateText
          :value="startDay * 86400"
          :end-value="endDay * 86400"
          utc
        />
        <svg
          class="range-bar__caret"
          viewBox="0 0 12 12"
          aria-hidden="true"
        ><path
          d="M2.5 4.5 6 8l3.5-3.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.4"
        /></svg>
      </button>
      <div
        v-if="open"
        class="range-bar__popover"
        role="dialog"
        :aria-label="t('analytics.ranges.custom')"
      >
        <BaseDateRange
          v-model:start="start"
          v-model:end="end"
          size="sm"
          :from-label="t('analytics.ranges.from')"
          :to-label="t('analytics.ranges.to')"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.range-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}

.range-bar__custom {
  position: relative;
}

.range-bar__trigger {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: var(--control-height-sm);
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
  cursor: pointer;
}

.range-bar__trigger:hover,
.range-bar__trigger[aria-expanded='true'] {
  border-color: var(--color-border-strong);
}

.range-bar__caret {
  width: 12px;
  height: 12px;
  color: var(--color-text-subtle);
}

.range-bar__popover {
  position: absolute;
  z-index: var(--z-dropdown);
  top: calc(100% + var(--space-2));
  left: 0;
  padding: var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
  box-shadow: var(--shadow-popup);
}

@media (width < 600px) {
  .range-bar__popover {
    position: fixed;
    right: var(--space-4);
    left: var(--space-4);
  }
}
</style>
