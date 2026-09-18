<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import type { AnalyticsSummaryData } from '../../../shared/messages.ts';
import { IconBarChart } from '../icons.vue';
import { Card, Chip, ChipGroup } from '../ui/Card.vue';
import { AnalyticsChart } from './AnalyticsChart.vue';
import { AnalyticsDayBreakdown } from './AnalyticsDayBreakdown.vue';
import { AnalyticsHourlyChart } from './AnalyticsHourlyChart.vue';
import {
  ANALYTICS_METRICS,
  METRIC_COLOR_VAR,
  isSingleDaySpan,
  type AnalyticsMetric,
} from './analytics-chart.ts';
import {
  buildHourlySeries,
  formatDayTimeSubtitle,
  hourRowsToSeries,
} from './analytics-intraday.ts';
import { tzOffsetSecsForDay } from './analytics-range.ts';

type AnalyticsActivityCardProps = {
  locale: Locale;
  summary: AnalyticsSummaryData;
  metric: AnalyticsMetric;
  onMetricChange: (metric: AnalyticsMetric) => void;
};

const METRIC_LABELS: Record<AnalyticsMetric, 'analyticsChats' | 'analyticsGifts' | 'analyticsLikes' | 'analyticsDiamonds' | 'analyticsViewers'> = {
  chats: 'analyticsChats',
  gifts: 'analyticsGifts',
  likes: 'analyticsLikes',
  diamonds: 'analyticsDiamonds',
  peakViewers: 'analyticsViewers',
};

/**
 * Activity card that works for every range:
 * - multi-day spans render the daily trend line (AnalyticsChart) in the
 *   selected metric's color,
 * - single-day spans (Today preset or a custom From==To day, in the system
 *   zone) render a stacked multi-metric hourly graphic with legend plus the
 *   per-metric breakdown, with the local date and zone in the subtitle.
 * The hourly chart prefers the backend's true per-hour counters and falls
 * back to the last-activity approximation for pre-hourly data.
 * Metric switching reuses the shared Chip/ChipGroup primitives.
 */
export const AnalyticsActivityCard = defineVueFunctional<AnalyticsActivityCardProps>((props) => {
  const { locale, summary, metric, onMetricChange } = props;
  const singleDay = isSingleDaySpan(summary.startDay, summary.endDay);
  const offset = singleDay ? tzOffsetSecsForDay(summary.startDay) : 0;
  const series: Record<AnalyticsMetric, number[]> = singleDay
    ? summary.hours.length > 0
      ? hourRowsToSeries(summary.hours)
      : {
          chats: buildHourlySeries(summary.topViewers, summary.startDay, 'chats', offset),
          gifts: buildHourlySeries(summary.topViewers, summary.startDay, 'gifts', offset),
          likes: buildHourlySeries(summary.topViewers, summary.startDay, 'likes', offset),
          diamonds: buildHourlySeries(summary.topViewers, summary.startDay, 'diamonds', offset),
          peakViewers: buildHourlySeries(summary.topViewers, summary.startDay, 'peakViewers', offset),
        }
    : { chats: [], gifts: [], likes: [], diamonds: [], peakViewers: [] };

  return (
    <Card
      title={singleDay ? t(locale, 'analyticsTodayTimeline') : t(locale, 'analyticsActivity')}
      subtitle={singleDay ? formatDayTimeSubtitle(summary.startDay, locale) : undefined}
      hint={singleDay ? t(locale, 'analyticsHourlyHint') : undefined}
      icon={<IconBarChart />}
    >
      <div class="analytics-activity" style={{ '--metric-color': METRIC_COLOR_VAR[metric] }}>
        <ChipGroup>
          {ANALYTICS_METRICS.map((key) => (
            <Chip key={key} active={metric === key} onClick={() => onMetricChange(key)}>
              {t(locale, METRIC_LABELS[key])}
            </Chip>
          ))}
        </ChipGroup>
        {singleDay ? (
          <>
            <AnalyticsHourlyChart
              locale={locale}
              series={series}
              active={metric}
              metricLabel={(key) => t(locale, METRIC_LABELS[key])}
            />
            <AnalyticsDayBreakdown
              locale={locale}
              day={summary.startDay}
              totals={summary.totals}
              metric={metric}
              metricLabel={(key) => t(locale, METRIC_LABELS[key])}
              onMetricChange={onMetricChange}
            />
          </>
        ) : (
          <AnalyticsChart
            locale={locale}
            days={summary.days}
            startDay={summary.startDay}
            endDay={summary.endDay}
            metric={metric}
            label={t(locale, METRIC_LABELS[metric])}
          />
        )}
      </div>
    </Card>
  );
});

export default AnalyticsActivityCard;
</script>
