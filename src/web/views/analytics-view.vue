<script lang="tsx">
import { computed, ref, watch } from 'vue';

import { IconBarChart } from '../components/icons.vue';
import { isSingleDaySpan, type AnalyticsMetric } from '../components/analytics/analytics-chart.ts';
import { AnalyticsActivityCard } from '../components/analytics/AnalyticsActivityCard.vue';
import { AnalyticsContributorsCard } from '../components/analytics/AnalyticsContributorsCard.vue';
import { AnalyticsRangeFilter } from '../components/analytics/AnalyticsRangeFilter.vue';
import { AnalyticsSessionsCard } from '../components/analytics/AnalyticsSessionsCard.vue';
import { AnalyticsStatsGrid } from '../components/analytics/AnalyticsStatsGrid.vue';
import { formatDayTimeSubtitle } from '../components/analytics/analytics-intraday.ts';
import {
  dayToIsoDate,
  isoDateToDay,
  resolveAnalyticsRange,
  systemDay,
  tzOffsetSecsForDay,
  type AnalyticsRangeKey,
} from '../components/analytics/analytics-range.ts';
import { Badge, Card, Chip, ChipGroup, EmptyState } from '../components/ui/Card.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { SearchInput } from '../components/ui/TextInput.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { AnalyticsSummaryData } from '../../shared/messages.ts';

type AnalyticsViewProps = {
  locale: Locale;
  creator: string;
  summary: AnalyticsSummaryData | null;
  onRequestRange: (startDay: number, endDay: number, tzOffsetSecs: number) => void;
};

type AnalyticsTab = 'overview' | 'engagement' | 'contributors' | 'sessions';

export const AnalyticsView = defineVueComponent<AnalyticsViewProps>(
  ['locale', 'creator', 'summary', 'onRequestRange'],
  (props) => {
  const range = ref<AnalyticsRangeKey>('7d');
  const metric = ref<AnalyticsMetric>('chats');
  const tab = ref<AnalyticsTab>('overview');
  const contributorQuery = ref('');
  // Date inputs work on system-local calendar labels.
  const customStart = ref(dayToIsoDate(systemDay(Date.now()) - 6));
  const customEnd = ref(dayToIsoDate(systemDay(Date.now())));

  const request = (): void => {
    // Today follows the system clock, and the zone offset travels with the
    // request so the host buckets days and hours in the same frame.
    const today = systemDay(Date.now());
    const span = resolveAnalyticsRange(range.value, today, {
      startDay: isoDateToDay(customStart.value) ?? today,
      endDay: isoDateToDay(customEnd.value) ?? today,
    });
    props.onRequestRange(span.startDay, span.endDay, tzOffsetSecsForDay(span.endDay));
  };
  watch([range, customStart, customEnd, () => props.creator], request, { immediate: true });

  const filteredContributors = computed(() => {
    const q = contributorQuery.value.trim().toLowerCase();
    const rows = props.summary?.topViewers ?? [];
    if (!q) return rows;
    return rows.filter((row) => row.uniqueId.toLowerCase().includes(q));
  });

  return () => {
    const { locale, creator, summary } = props;
    const totals = summary?.totals;
    const hasData = summary !== null && (
      summary.days.length > 0
      || summary.topViewers.length > 0
      || summary.sessions > 0
      || (summary.hours ?? []).some((row) => Object.values(row).some((value) => typeof value === 'number' && value > 0))
      || (totals !== undefined && Object.values(totals).some((value) => value > 0))
    );
    const emptySingleDay = summary !== null && isSingleDaySpan(summary.startDay, summary.endDay);
    const tabs: Array<{ key: AnalyticsTab; label: string }> = [
      { key: 'overview', label: t(locale, 'analyticsTabOverview') },
      { key: 'engagement', label: t(locale, 'analyticsTabEngagement') },
      { key: 'contributors', label: t(locale, 'analyticsTabContributors') },
      { key: 'sessions', label: t(locale, 'analyticsTabSessions') },
    ];

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
          <Card
            title={emptySingleDay ? t(locale, 'analyticsTodayTimeline') : t(locale, 'analyticsActivity')}
            subtitle={emptySingleDay ? formatDayTimeSubtitle(summary.startDay, locale) : undefined}
            icon={<IconBarChart />}
          >
            <EmptyState
              title={t(locale, 'analyticsEmpty')}
              description={
                emptySingleDay
                  ? t(locale, 'analyticsEmptyTodayHint', { date: formatDayTimeSubtitle(summary.startDay, locale) })
                  : t(locale, 'analyticsEmptyHint')
              }
            />
          </Card>
        ) : (
          <>
            <div class="analytics-tabs" role="tablist">
              <ChipGroup>
                {tabs.map((entry) => (
                  <Chip key={entry.key} active={tab.value === entry.key} onClick={() => { tab.value = entry.key; }}>
                    {entry.label}
                  </Chip>
                ))}
              </ChipGroup>
            </div>

            {tab.value === 'overview' ? (
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
            ) : null}

            {tab.value === 'engagement' ? (
              <div class="analytics-engagement-tab">
                <p class="analytics-tab-hint">{t(locale, 'analyticsEngagementHint')}</p>
                <AnalyticsActivityCard
                  locale={locale}
                  summary={summary}
                  metric={metric.value}
                  onMetricChange={(next) => { metric.value = next; }}
                />
              </div>
            ) : null}

            {tab.value === 'contributors' ? (
              <div class="analytics-contributors-tab">
                <SearchInput
                  value={contributorQuery.value}
                  onValueChange={(next) => { contributorQuery.value = next; }}
                  placeholder={t(locale, 'analyticsSearchContributors')}
                />
                <AnalyticsContributorsCard locale={locale} topViewers={filteredContributors.value} />
              </div>
            ) : null}

            {tab.value === 'sessions' ? (
              <div class="analytics-sessions-tab">
                <AnalyticsSessionsCard locale={locale} summary={summary} />
              </div>
            ) : null}
          </>
        )}
      </Page>
    );
  };
  },
);

export default AnalyticsView;
</script>
