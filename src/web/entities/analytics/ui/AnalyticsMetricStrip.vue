<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsDayRow, AnalyticsTotals } from '../../../../shared/messages.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import { fillDaySeries, metricColor, analyticsMetrics, type AnalyticsMetric } from '../lib/analytics-metrics.ts';
import { buildLineGeometry } from '../lib/chart-geometry.ts';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconHeart from '@/shared/ui/icon/icons/IconHeart.vue';
import IconDiamond from '@/shared/ui/icon/icons/IconDiamond.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';

/**
 * Summary KPI cards: value, delta against the previous same-length period and
 * a sparkline of the daily series. They are display-only — the chart metric is
 * picked in the activity panel itself.
 */
const props = defineProps<{
  totals: AnalyticsTotals;
  previousTotals: AnalyticsTotals | null;
  days: readonly AnalyticsDayRow[];
  startDay: number;
  endDay: number;
}>();

const { t, locale } = useI18n();
const icons: Record<AnalyticsMetric, typeof IconChat> = {
  chats: IconChat,
  gifts: IconCoins,
  likes: IconHeart,
  diamonds: IconDiamond,
  peakViewers: IconEye,
};
const sparkBox = { width: 120, height: 28, pad: 1 };

interface Delta {
  text: string;
  tone: 'up' | 'down' | 'flat' | 'none';
}

function deltaOf(metric: AnalyticsMetric): Delta {
  const current = Math.max(0, props.totals[metric] ?? 0);
  const before = props.previousTotals ? Math.max(0, props.previousTotals[metric] ?? 0) : null;
  if (before === null) return { text: '', tone: 'none' };
  if (before === 0 && current === 0) return { text: '—', tone: 'flat' };
  if (before === 0) return { text: `▲ ${t('analytics.kpi.new')}`, tone: 'up' };
  const ratio = (current - before) / before;
  if (Math.abs(ratio) < 0.005) return { text: '=', tone: 'flat' };
  const percent = `${Math.abs(Math.round(ratio * 100))}%`;
  return ratio > 0 ? { text: `▲ ${percent}`, tone: 'up' } : { text: `▼ ${percent}`, tone: 'down' };
}

const cards = computed(() =>
  analyticsMetrics.map((metric) => {
    const series = fillDaySeries(props.days, props.startDay, props.endDay, metric);
    const geometry = buildLineGeometry(series, sparkBox);
    return {
      metric,
      label: t(`analytics.metrics.${metric}`),
      value: formatCount(props.totals[metric] ?? 0, locale.value),
      delta: deltaOf(metric),
      spark: geometry.line,
      empty: (props.totals[metric] ?? 0) === 0,
    };
  }),
);
</script>

<template>
  <div
    class="kpi-strip"
    role="list"
    :aria-label="t('analytics.metricPicker')"
  >
    <div
      v-for="card in cards"
      :key="card.metric"
      class="kpi-strip__card"
      :class="{ 'kpi-strip__card--empty': card.empty }"
      role="listitem"
      :style="{ '--metric-color': metricColor[card.metric] }"
    >
      <span class="kpi-strip__head">
        <component
          :is="icons[card.metric]"
          :size="14"
        />
        <span class="kpi-strip__label">{{ card.label }}</span>
        <span
          v-if="card.delta.text"
          class="kpi-strip__delta"
          :class="`kpi-strip__delta--${card.delta.tone}`"
          :title="t('analytics.kpi.vsPrevious')"
        >{{ card.delta.text }}</span>
      </span>
      <span class="kpi-strip__value">{{ card.value }}</span>
      <svg
        class="kpi-strip__spark"
        :viewBox="`0 0 ${sparkBox.width} ${sparkBox.height}`"
        aria-hidden="true"
      >
        <path
          v-if="card.spark"
          :d="card.spark"
        />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.kpi-strip {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--space-4);
}

.kpi-strip__card {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
  padding: var(--space-4) var(--space-5);
  border: var(--border-width) solid var(--color-border-subtle);
  border-top: 2px solid var(--metric-color);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
}

.kpi-strip__card--empty {
  opacity: .55;
}

.kpi-strip__head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.kpi-strip__head svg {
  color: var(--metric-color);
  flex: none;
}

.kpi-strip__label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.kpi-strip__delta {
  margin-left: auto;
  flex: none;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
  cursor: default;
}

.kpi-strip__delta--up { color: var(--color-text-success); }
.kpi-strip__delta--down { color: var(--color-text-danger); }
.kpi-strip__delta--flat { color: var(--color-text-subtle); }

.kpi-strip__value {
  font-size: 20px;
  font-weight: var(--font-weight-semibold);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.01em;
}

.kpi-strip__spark {
  width: 100%;
  height: 28px;
}

.kpi-strip__spark path {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 1.5;
  stroke-linejoin: round;
  stroke-linecap: round;
  opacity: .8;
}

@media (width < 1100px) {
  .kpi-strip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (width < 720px) {
  .kpi-strip {
    display: flex;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    padding-bottom: var(--space-2);
  }

  .kpi-strip__card {
    flex: 0 0 150px;
    scroll-snap-align: start;
  }
}
</style>
