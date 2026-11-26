<script setup lang="ts">
/** Floating value readout shared by the hand-rolled SVG charts: one row per
 *  visible series with its color swatch, plus an optional previous-period
 *  reference value. Positioned by the host chart through the style attr. */
defineProps<{
  title: string;
  rows: readonly { key: string; color: string; label: string; value: string; before: string | null }[];
  compareLabel?: string;
}>();
</script>

<template>
  <div
    class="chart-tooltip"
    role="presentation"
  >
    <div class="chart-tooltip__title">
      {{ title }}
    </div>
    <div
      v-for="row in rows"
      :key="row.key"
      class="chart-tooltip__row"
    >
      <span
        class="chart-tooltip__swatch"
        :style="{ background: row.color }"
      />
      <span class="chart-tooltip__label">{{ row.label }}</span>
      <span class="chart-tooltip__value">{{ row.value }}</span>
      <span
        v-if="row.before !== null"
        class="chart-tooltip__before"
        :title="compareLabel"
      >{{ row.before }}</span>
    </div>
  </div>
</template>

<style scoped>
.chart-tooltip {
  position: absolute;
  z-index: var(--z-dropdown);
  top: var(--space-2);
  min-width: 140px;
  padding: var(--space-2) var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
  box-shadow: var(--shadow-popup);
  font-size: var(--font-size-sm);
  pointer-events: none;
}

.chart-tooltip__title {
  margin-bottom: var(--space-1);
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
}

.chart-tooltip__row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.chart-tooltip__swatch {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-sm, 3px);
}

.chart-tooltip__label {
  flex: 1;
  min-width: 0;
  color: var(--color-text-muted);
  white-space: nowrap;
}

.chart-tooltip__value {
  font-weight: var(--font-weight-semibold, 600);
  font-variant-numeric: tabular-nums;
}

.chart-tooltip__before {
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}
</style>
