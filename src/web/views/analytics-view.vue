<script lang="tsx">
import { ref, watch } from 'vue';

import { IconBarChart } from '../components/icons.vue';
import type { AnalyticsMetric } from '../components/analytics/analytics-chart.ts';
import { AnalyticsActivityCard } from '../components/analytics/AnalyticsActivityCard.vue';
import { AnalyticsContributorsCard } from '../components/analytics/AnalyticsContributorsCard.vue';
import { AnalyticsRangeFilter } from '../components/analytics/AnalyticsRangeFilter.vue';
import { AnalyticsSessionsCard } from '../components/analytics/AnalyticsSessionsCard.vue';
import { AnalyticsStatsGrid } from '../components/analytics/AnalyticsStatsGrid.vue';
import {
  dayToIsoDate,
  isoDateToDay,
  resolveAnalyticsRange,
  utcDay,
  type AnalyticsRangeKey,
} from '../components/analytics/analytics-range.ts';
import { Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { AnalyticsSummaryData } from '../../shared/messages.ts';

type AnalyticsViewProps = {
  locale: Locale;
  creator: string;
  summary: AnalyticsSummaryData | null;
  onRequestRange: (startDay: number, endDay: number) => void;
};

export const AnalyticsView = defineVueComponent<AnalyticsViewProps>(
  ['locale', 'creator', 'summary', 'onRequestRange'],
  (props) => {
  const range = ref<AnalyticsRangeKey>('7d');
  const metric = ref<AnalyticsMetric>('chats');
  const customStart = ref(dayToIsoDate(utcDay(Date.now()) - 6));
  const customEnd = ref(dayToIsoDate(utcDay(Date.now())));

  const request = (): void => {
    const today = utcDay(Date.now());
    const span = resolveAnalyticsRange(range.value, today, {
      startDay: isoDateToDay(customStart.value) ?? today,
      endDay: isoDateToDay(customEnd.value) ?? today,
    });
    props.onRequestRange(span.startDay, span.endDay);
  };
  watch([range, customStart, customEnd, () => props.creator], request, { immediate: true });

  return () => {
    const { locale, creator, summary } = props;
    const totals = summary?.totals;
    const hasData = summary !== null && (
      summary.days.length > 0
      || summary.topViewers.length > 0
      || summary.sessions > 0
      || (totals !== undefined && Object.values(totals).some((value) => value > 0))
    );

    return (
      <Page width="wide">
        <PageHeader
          title={t(locale, 'tabAnalytics')}
          icon={<IconBarChart />}
          meta={creator ? <Badge>@{creator}</Badge> : null}
        />

        <AnalyticsRangeFilter
          locale={locale}
          range={range.value}
          customStart={customStart.value}
          customEnd={customEnd.value}
          onRangeChange={(next) => { range.value = next; }}
          onCustomStartChange={(next) => { customStart.value = next; }}
          onCustomEndChange={(next) => { customEnd.value = next; }}
        />

        {!summary ? null : !hasData ? (
          <Card title={t(locale, 'analyticsActivity')} icon={<IconBarChart />}>
            <EmptyState title={t(locale, 'analyticsEmpty')} description={t(locale, 'analyticsEmptyHint')} />
          </Card>
        ) : (
          <>
            {totals ? <AnalyticsStatsGrid locale={locale} totals={totals} sessions={summary.sessions} /> : null}

            <div class="analytics-main">
              <AnalyticsActivityCard
                locale={locale}
                summary={summary}
                metric={metric.value}
                onMetricChange={(next) => { metric.value = next; }}
              />
              <AnalyticsSessionsCard locale={locale} summary={summary} />
            </div>

            <AnalyticsContributorsCard locale={locale} topViewers={summary.topViewers} />
          </>
        )}
      </Page>
    );
  };
  },
);

export default AnalyticsView;
</script>
