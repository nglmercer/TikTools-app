<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsDayRow, AnalyticsTotals } from '../../../../shared/messages.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import { useAnchoredPosition } from '@/shared/lib/use-anchored-position.ts';
import { fillDaySeries, metricColor, analyticsMetrics, type AnalyticsMetric } from '../lib/analytics-metrics.ts';
import { formatDayLabel } from '../lib/analytics-range.ts';
import { buildLineGeometry } from '../lib/chart-geometry.ts';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconHeart from '@/shared/ui/icon/icons/IconHeart.vue';
import IconDiamond from '@/shared/ui/icon/icons/IconDiamond.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';

/**
 * Summary KPI cards: value and delta against the previous same-length period.
 * No inline charts — the single miniature lives in the preview that hovering
 * (fine pointers) or tapping (touch) floats over the card, so a metric's
 * shape never renders twice on the page.
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
const previewBox = { width: 264, height: 96, pad: 8 };
const HOVER_DELAY_MS = 250;

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

function seriesOf(metric: AnalyticsMetric): number[] {
  return fillDaySeries(props.days, props.startDay, props.endDay, metric);
}

const cards = computed(() =>
  analyticsMetrics.map((metric) => ({
    metric,
    label: t(`analytics.metrics.${metric}`),
    value: formatCount(props.totals[metric] ?? 0, locale.value),
    delta: deltaOf(metric),
    empty: (props.totals[metric] ?? 0) === 0,
  })),
);

// ── Preview ──────────────────────────────────────────────────────────────
// Hover-driven on fine pointers, tap-toggled on touch. A single floating
// panel anchors to the active card and repositions on scroll/resize.
const canHover = typeof window !== 'undefined' && window.matchMedia('(hover: hover) and (pointer: fine)').matches;
const previewMetric = ref<AnalyticsMetric | null>(null);
const previewOpen = ref(false);
const anchorElement = ref<HTMLElement | null>(null);
const floatingElement = ref<HTMLElement | null>(null);
const cardElements = new Map<AnalyticsMetric, HTMLElement>();
let hoverTimer: ReturnType<typeof setTimeout> | null = null;
const { style: floatingStyle } = useAnchoredPosition(anchorElement, floatingElement, previewOpen, {
  placement: 'bottom-start',
  offset: 8,
});

function collectCard(metric: AnalyticsMetric, element: unknown): void {
  if (element instanceof HTMLElement) cardElements.set(metric, element);
  else cardElements.delete(metric);
}

function activate(metric: AnalyticsMetric): void {
  anchorElement.value = cardElements.get(metric) ?? null;
  previewMetric.value = metric;
  previewOpen.value = true;
}

function cancelHover(): void {
  if (hoverTimer) clearTimeout(hoverTimer);
  hoverTimer = null;
}

function close(): void {
  cancelHover();
  previewOpen.value = false;
  previewMetric.value = null;
}

function onEnter(metric: AnalyticsMetric): void {
  if (!canHover) return;
  cancelHover();
  hoverTimer = setTimeout(() => activate(metric), HOVER_DELAY_MS);
}

function onLeave(): void {
  if (!canHover) return;
  close();
}

function onTap(metric: AnalyticsMetric): void {
  if (canHover) return;
  if (previewMetric.value === metric && previewOpen.value) close();
  else activate(metric);
}

function onDocumentClick(event: MouseEvent): void {
  if (canHover || !previewOpen.value) return;
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (floatingElement.value?.contains(target)) return;
  if ([...cardElements.values()].some((element) => element.contains(target))) return;
  close();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') close();
}

document.addEventListener('mousedown', onDocumentClick);
document.addEventListener('keydown', onKeydown);
onBeforeUnmount(() => {
  cancelHover();
  document.removeEventListener('mousedown', onDocumentClick);
  document.removeEventListener('keydown', onKeydown);
});

const preview = computed(() => {
  const metric = previewMetric.value;
  if (!metric) return null;
  const series = seriesOf(metric);
  const geometry = buildLineGeometry(series, previewBox);
  const active = series.filter((value) => value > 0);
  const peakIndex = series.indexOf(Math.max(...series));
  const peak = Math.max(...series);
  return {
    metric,
    label: t(`analytics.metrics.${metric}`),
    total: formatCount(props.totals[metric] ?? 0, locale.value),
    delta: deltaOf(metric),
    line: geometry.line,
    area: geometry.area,
    average: formatCount(Math.round((props.totals[metric] ?? 0) / Math.max(1, series.length)), locale.value),
    bestDay: peak > 0 && peakIndex >= 0 ? `${formatDayLabel(props.startDay + peakIndex, locale.value)} · ${formatCount(peak, locale.value)}` : '—',
    idle: active.length === 0,
  };
});
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
      :ref="(element) => collectCard(card.metric, element)"
      class="kpi-strip__card"
      :class="{ 'kpi-strip__card--empty': card.empty, 'kpi-strip__card--previewing': previewMetric === card.metric }"
      role="listitem"
      :style="{ '--metric-color': metricColor[card.metric] }"
      @pointerenter="onEnter(card.metric)"
      @pointerleave="onLeave"
      @click="onTap(card.metric)"
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
    </div>
  </div>

  <Teleport to="body">
    <div
      v-if="preview && previewOpen"
      ref="floatingElement"
      class="kpi-preview"
      :style="[floatingStyle, { '--metric-color': metricColor[preview.metric] }]"
      role="tooltip"
    >
      <div class="kpi-preview__head">
        <component
          :is="icons[preview.metric]"
          :size="14"
        />
        <span class="kpi-preview__label">{{ preview.label }}</span>
        <span class="kpi-preview__total">{{ preview.total }}</span>
        <span
          v-if="preview.delta.text"
          class="kpi-strip__delta"
          :class="`kpi-strip__delta--${preview.delta.tone}`"
        >{{ preview.delta.text }}</span>
      </div>
      <svg
        class="kpi-preview__chart"
        :viewBox="`0 0 ${previewBox.width} ${previewBox.height}`"
        role="img"
        :aria-label="`${preview.label}: ${preview.total}`"
      >
        <path
          v-if="preview.area"
          class="kpi-preview__area"
          :d="preview.area"
        />
        <path
          v-if="preview.line"
          class="kpi-preview__line"
          :d="preview.line"
        />
      </svg>
      <dl class="kpi-preview__stats">
        <div>
          <dt>{{ t('analytics.kpi.average') }}</dt>
          <dd>{{ preview.average }}</dd>
        </div>
        <div>
          <dt>{{ t('analytics.insights.bestDay') }}</dt>
          <dd>{{ preview.bestDay }}</dd>
        </div>
      </dl>
    </div>
  </Teleport>
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
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  border-top: 2px solid var(--metric-color);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
}

@media (hover: none) {
  .kpi-strip__card {
    cursor: pointer;
  }
}

.kpi-strip__card--empty {
  opacity: 0.55;
}

.kpi-strip__card--previewing {
  border-color: var(--color-border-strong);
  border-top-color: var(--metric-color);
}

.kpi-strip__head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--metric-color);
  font-size: var(--font-size-sm);
}

.kpi-strip__label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.kpi-strip__delta {
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

.kpi-strip__delta--up {
  color: var(--color-text-success);
}

.kpi-strip__delta--down {
  color: var(--color-text-danger, var(--color-text-warning));
}

.kpi-strip__delta--flat,
.kpi-strip__delta--none {
  color: var(--color-text-subtle);
}

.kpi-strip__value {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold, 600);
  font-variant-numeric: tabular-nums;
}

.kpi-preview {
  z-index: var(--z-dropdown);
  width: 280px;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-top: 2px solid var(--metric-color);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
  box-shadow: var(--shadow-popup);
}

.kpi-preview__head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--metric-color);
}

.kpi-preview__label {
  flex: 1;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.kpi-preview__total {
  font-weight: var(--font-weight-semibold, 600);
  font-variant-numeric: tabular-nums;
}

.kpi-preview__chart {
  display: block;
  width: 100%;
  margin-top: var(--space-2);
}

.kpi-preview__area {
  fill: var(--metric-color);
  opacity: 0.14;
}

.kpi-preview__line {
  fill: none;
  stroke: var(--metric-color);
  stroke-width: 1.8;
}

.kpi-preview__stats {
  display: flex;
  gap: var(--space-5);
  margin: var(--space-2) 0 0;
}

.kpi-preview__stats dt {
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
}

.kpi-preview__stats dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
}

@media (width < 1100px) {
  .kpi-strip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (width < 720px) {
  .kpi-strip {
    display: flex;
    gap: var(--space-3);
    overflow-x: auto;
    scroll-snap-type: x proximity;
    padding-bottom: var(--space-1);
  }

  .kpi-strip__card {
    flex: 0 0 132px;
    scroll-snap-align: start;
  }
}
</style>
