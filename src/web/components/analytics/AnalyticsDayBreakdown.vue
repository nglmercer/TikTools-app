<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import type { AnalyticsTotals } from '../../../shared/messages.ts';
import { buildMetricBreakdown, type AnalyticsMetric } from './analytics-chart.ts';
import { formatDayDate } from './analytics-intraday.ts';
import { formatCount } from './analytics-range.ts';

type AnalyticsDayBreakdownProps = {
  locale: Locale;
  day: number;
  totals: AnalyticsTotals;
  metric: AnalyticsMetric;
  metricLabel: (metric: AnalyticsMetric) => string;
  onMetricChange?: (metric: AnalyticsMetric) => void;
};

/**
 * Single-day totals below the Today hourly graphic. Every metric renders as
 * a normalized horizontal bar with the active metric highlighted.
 * No chart library — pure divs. All copy goes through i18n.
 */
export const AnalyticsDayBreakdown = defineVueFunctional<AnalyticsDayBreakdownProps>((props) => {
  const { locale, day, totals, metric, metricLabel, onMetricChange } = props;
  const entries = buildMetricBreakdown({
    chats: totals.chats,
    gifts: totals.gifts,
    likes: totals.likes,
    diamonds: totals.diamonds,
    peakViewers: totals.peakViewers,
  });

  return (
    <div class="analytics-single">
      <div class="analytics-single__head">
        <span class="analytics-single__day">{formatDayDate(day, locale)}</span>
        <span class="analytics-single__hint">
          {t(locale, 'analyticsEventsCount', { count: formatCount(totals.chats + totals.gifts + totals.likes, locale) })}
        </span>
      </div>
      <div class="analytics-bars" role="list">
        {entries.map((entry) => {
          const active = entry.metric === metric;
          return (
            <button
              key={entry.metric}
              type="button"
              role="listitem"
              class={`analytics-bar-row${active ? ' is-active' : ''}`}
              aria-pressed={active}
              onClick={onMetricChange ? () => onMetricChange(entry.metric) : undefined}
              style={{ cursor: onMetricChange ? 'pointer' : 'default' }}
            >
              <span class="analytics-bar-row__label">{metricLabel(entry.metric)}</span>
              <span class="analytics-bar-row__track">
                <span class="analytics-bar-row__fill" style={{ width: `${Math.round(entry.fraction * 100)}%` }} />
              </span>
              <span class="analytics-bar-row__value">{formatCount(entry.value, locale)}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
});

export default AnalyticsDayBreakdown;
</script>
