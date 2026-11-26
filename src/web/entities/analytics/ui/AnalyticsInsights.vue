<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AnalyticsSummaryData } from '../../../../shared/messages.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import BaseDateText from '@/shared/ui/date-text/BaseDateText.vue';
import { formatHourLabel } from '../lib/analytics-metrics.ts';
import { hourTotals, summarizeDaySeries, summarizeHourlySeries } from '../lib/analytics-insights.ts';

/**
 * Derived answers instead of raw counters: best day, peak hour, per-session
 * averages and the engagement mix over event counts (comparable units — raw
 * like/gift quantities are not).
 */
const props = defineProps<{
  summary: AnalyticsSummaryData;
}>();

const { t, locale } = useI18n();

const chats = computed(() => summarizeDaySeries(props.summary.days, props.summary.startDay, props.summary.endDay, 'chats'));
const peakHour = computed(() => {
  if (!props.summary.hours.length) return null;
  const result = summarizeHourlySeries(hourTotals(props.summary.hours, 'chats'));
  return result.peakValue > 0 && result.peakHour !== null ? result : null;
});

const perSession = computed(() => {
  const sessions = props.summary.sessions;
  if (!sessions) return null;
  return {
    chats: Math.round((props.summary.totals.chats / sessions) * 10) / 10,
    likes: Math.round((props.summary.totals.likes / sessions) * 10) / 10,
  };
});

/** Event-count mix: chats, gift events and like events are comparable units. */
const mix = computed(() => {
  const entries = [
    { key: 'chats' as const, labelKey: 'analytics.metrics.chats', value: Math.max(0, props.summary.totals.chats), color: 'var(--color-chart-1)' },
    { key: 'likeEvents' as const, labelKey: 'analytics.facts.likeEvents', value: Math.max(0, props.summary.totals.likeEvents), color: 'var(--color-chart-3)' },
    { key: 'giftEvents' as const, labelKey: 'analytics.facts.giftEvents', value: Math.max(0, props.summary.totals.giftEvents), color: 'var(--color-chart-2)' },
  ];
  const total = entries.reduce((sum, entry) => sum + entry.value, 0);
  return {
    total,
    entries: entries.map((entry) => ({ ...entry, share: total > 0 ? entry.value / total : 0 })),
  };
});

const social = computed(() =>
  (['follows', 'shares', 'joins'] as const).map((key) => ({ key, label: t(`analytics.facts.${key}`), value: props.summary.totals[key] ?? 0 })),
);
</script>

<template>
  <div class="insights">
    <dl class="insights__rows">
      <div class="insights__row">
        <dt>{{ t('analytics.insights.bestDay') }}</dt>
        <dd>
          <template v-if="chats.peakDay !== null && chats.peakValue > 0">
            <BaseDateText
              :value="chats.peakDay * 86400"
              utc
            />
            · {{ formatCount(chats.peakValue, locale) }}
          </template>
          <template v-else>
            —
          </template>
        </dd>
      </div>
      <div class="insights__row">
        <dt>{{ t('analytics.insights.peakHour') }}</dt>
        <dd>{{ peakHour ? `${formatHourLabel(peakHour.peakHour ?? 0)} · ${formatCount(peakHour.peakValue, locale)}` : '—' }}</dd>
      </div>
      <div class="insights__row">
        <dt>{{ t('analytics.insights.perSession') }}</dt>
        <dd>
          <template v-if="perSession">
            {{ formatCount(perSession.chats, locale) }} {{ t('analytics.metrics.chats').toLowerCase() }} · {{ formatCount(perSession.likes, locale) }} {{ t('analytics.metrics.likes').toLowerCase() }}
          </template>
          <template v-else>
            —
          </template>
        </dd>
      </div>
      <div class="insights__row">
        <dt>{{ t('analytics.insights.sessions') }}</dt>
        <dd>{{ formatCount(summary.sessions, locale) }}</dd>
      </div>
    </dl>

    <div
      v-if="mix.total > 0"
      class="insights__mix"
    >
      <span class="insights__mix-label">{{ t('analytics.insights.mix') }}</span>
      <div
        class="insights__mix-bar"
        role="img"
        :aria-label="mix.entries.map((entry) => `${t(entry.labelKey)} ${Math.round(entry.share * 100)}%`).join(', ')"
      >
        <span
          v-for="entry in mix.entries"
          :key="entry.key"
          class="insights__mix-slice"
          :style="{ width: `${entry.share * 100}%`, background: entry.color }"
        />
      </div>
      <div class="insights__mix-legend">
        <span
          v-for="entry in mix.entries"
          :key="entry.key"
          class="insights__mix-entry"
        >
          <span
            class="insights__mix-dot"
            :style="{ background: entry.color }"
          />
          {{ t(entry.labelKey) }} {{ Math.round(entry.share * 100) }}%
        </span>
      </div>
    </div>

    <dl class="insights__rows insights__rows--social">
      <div
        v-for="row in social"
        :key="row.key"
        class="insights__row"
      >
        <dt>{{ row.label }}</dt>
        <dd>{{ formatCount(row.value, locale) }}</dd>
      </div>
    </dl>
  </div>
</template>

<style scoped>
.insights {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.insights__rows {
  margin: 0;
}

.insights__row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-5);
  padding: var(--space-2) var(--space-1);
}

.insights__row + .insights__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.insights__row dt {
  color: var(--color-text-muted);
}

.insights__row dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
  font-weight: var(--font-weight-semibold);
  text-align: right;
}

.insights__rows--social {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.insights__mix {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.insights__mix-label {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.insights__mix-bar {
  display: flex;
  height: 8px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-bg-control);
}

.insights__mix-slice:first-child {
  border-radius: 999px 0 0 999px;
}

.insights__mix-legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.insights__mix-entry {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

.insights__mix-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
</style>
