<script lang="tsx">
import { ref, watch } from 'vue';

import { IconBarChart, IconChat, IconCoins, IconGift, IconHeart, IconTrophy, IconUsers } from '../components/icons.vue';
import { AnalyticsChart } from '../components/analytics/AnalyticsChart.vue';
import {
  ANALYTICS_METRICS,
  type AnalyticsMetric,
} from '../components/analytics/analytics-chart.ts';
import {
  ANALYTICS_RANGE_KEYS,
  dayToIsoDate,
  formatCount,
  isoDateToDay,
  resolveAnalyticsRange,
  utcDay,
  type AnalyticsRangeKey,
} from '../components/analytics/analytics-range.ts';
import { Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { DatePicker } from '../components/ui/DatePicker.vue';
import { Page, PageHeader, StatCard, StatGrid } from '../components/ui/Page.vue';
import { DataTable, type Column } from '../components/ui/Table.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { AnalyticsSummaryData, AnalyticsTopViewer } from '../../shared/messages.ts';

type AnalyticsViewProps = {
  locale: Locale;
  creator: string;
  summary: AnalyticsSummaryData | null;
  onRequestRange: (startDay: number, endDay: number) => void;
};

const RANGE_LABELS: Record<AnalyticsRangeKey, 'rangeToday' | 'range7Days' | 'range30Days' | 'range90Days' | 'rangeCustom'> = {
  today: 'rangeToday',
  '7d': 'range7Days',
  '30d': 'range30Days',
  '90d': 'range90Days',
  custom: 'rangeCustom',
};

const METRIC_LABELS: Record<AnalyticsMetric, 'analyticsChats' | 'analyticsGifts' | 'analyticsLikes' | 'analyticsDiamonds' | 'analyticsViewers'> = {
  chats: 'analyticsChats',
  gifts: 'analyticsGifts',
  likes: 'analyticsLikes',
  diamonds: 'analyticsDiamonds',
  peakViewers: 'analyticsViewers',
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

    const viewerColumns: Column<AnalyticsTopViewer>[] = [
      {
        key: 'rank',
        header: t(locale, 'rank'),
        width: '64px',
        render: (_row, idx) => idx < 3 ? (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, fontWeight: 800, color: idx === 0 ? '#b45309' : idx === 1 ? '#64748b' : '#92400e' }}>
            <IconTrophy /> {idx + 1}
          </span>
        ) : (
          <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>#{idx + 1}</span>
        ),
      },
      {
        key: 'viewer',
        header: t(locale, 'viewer'),
        render: (row) => <span style={{ fontWeight: 600 }}>@{row.uniqueId}</span>,
      },
      {
        key: 'chats',
        header: t(locale, 'analyticsChats'),
        width: '90px',
        align: 'right' as const,
        render: (row) => <span style={{ fontWeight: 700 }}>{formatCount(row.chats, locale)}</span>,
      },
      {
        key: 'gifts',
        header: t(locale, 'analyticsGifts'),
        width: '90px',
        align: 'right' as const,
        render: (row) => <span style={{ fontWeight: 700 }}>{formatCount(row.gifts, locale)}</span>,
      },
      {
        key: 'diamonds',
        header: t(locale, 'analyticsDiamonds'),
        width: '100px',
        align: 'right' as const,
        render: (row) => <span style={{ fontWeight: 700 }}>{formatCount(row.diamonds, locale)}</span>,
      },
      {
        key: 'interactions',
        header: t(locale, 'analyticsInteractions'),
        width: '110px',
        align: 'right' as const,
        render: (row) => <span style={{ fontWeight: 700 }}>{formatCount(row.interactions, locale)}</span>,
      },
    ];

    const facts = totals ? [
      { label: t(locale, 'analyticsSessions'), value: formatCount(summary?.sessions ?? 0, locale) },
      { label: t(locale, 'analyticsPeakViewers'), value: formatCount(totals.peakViewers, locale) },
      { label: t(locale, 'analyticsGiftEvents'), value: formatCount(totals.giftEvents, locale) },
      { label: t(locale, 'analyticsLikeEvents'), value: formatCount(totals.likeEvents, locale) },
      { label: t(locale, 'analyticsFollows'), value: formatCount(totals.follows, locale) },
      { label: t(locale, 'analyticsShares'), value: formatCount(totals.shares, locale) },
      { label: t(locale, 'analyticsJoins'), value: formatCount(totals.joins, locale) },
    ] : [];

    return (
      <Page width="wide">
        <PageHeader
          title={t(locale, 'tabAnalytics')}
          icon={<IconBarChart />}
          meta={creator ? <Badge>@{creator}</Badge> : null}
        />

        <div class="analytics-toolbar">
          <div class="analytics-chips" role="group">
            {ANALYTICS_RANGE_KEYS.map((key) => (
              <button
                key={key}
                type="button"
                class={`analytics-chip${range.value === key ? ' is-active' : ''}`}
                aria-pressed={range.value === key}
                onClick={() => { range.value = key; }}
              >
                {t(locale, RANGE_LABELS[key])}
              </button>
            ))}
          </div>
          {range.value === 'custom' ? (
            <div class="analytics-custom">
              <DatePicker
                name="analytics-start"
                label={t(locale, 'rangeFrom')}
                value={customStart.value}
                onValueChange={(next) => { customStart.value = next; }}
                max={customEnd.value}
              />
              <DatePicker
                name="analytics-end"
                label={t(locale, 'rangeTo')}
                value={customEnd.value}
                onValueChange={(next) => { customEnd.value = next; }}
                min={customStart.value}
              />
            </div>
          ) : null}
        </div>

        {!summary ? null : !hasData ? (
          <Card title={t(locale, 'analyticsActivity')} icon={<IconBarChart />}>
            <EmptyState title={t(locale, 'analyticsEmpty')} description={t(locale, 'analyticsEmptyHint')} />
          </Card>
        ) : (
          <>
            <StatGrid>
              <StatCard icon={<IconChat />} value={formatCount(totals?.chats ?? 0, locale)} label={t(locale, 'analyticsChats')} tone="cyan" />
              <StatCard icon={<IconGift />} value={formatCount(totals?.gifts ?? 0, locale)} label={t(locale, 'analyticsGifts')} tone="pink" />
              <StatCard icon={<IconHeart />} value={formatCount(totals?.likes ?? 0, locale)} label={t(locale, 'analyticsLikes')} tone="yellow" />
              <StatCard icon={<IconUsers />} value={formatCount(totals?.peakViewers ?? 0, locale)} label={t(locale, 'analyticsPeakViewers')} tone="green" />
              <StatCard icon={<IconCoins />} value={formatCount(totals?.diamonds ?? 0, locale)} label={t(locale, 'analyticsDiamonds')} tone="cyan" />
              <StatCard icon={<IconBarChart />} value={formatCount(summary.sessions, locale)} label={t(locale, 'analyticsSessions')} tone="pink" />
            </StatGrid>

            <div class="analytics-main">
              <Card title={t(locale, 'analyticsActivity')} icon={<IconBarChart />}>
                <div class="analytics-chips" role="group">
                  {ANALYTICS_METRICS.map((key) => (
                    <button
                      key={key}
                      type="button"
                      class={`analytics-chip${metric.value === key ? ' is-active' : ''}`}
                      aria-pressed={metric.value === key}
                      onClick={() => { metric.value = key; }}
                    >
                      {t(locale, METRIC_LABELS[key])}
                    </button>
                  ))}
                </div>
                <AnalyticsChart
                  locale={locale}
                  days={summary.days}
                  startDay={summary.startDay}
                  endDay={summary.endDay}
                  metric={metric.value}
                  label={t(locale, METRIC_LABELS[metric.value])}
                />
              </Card>

              <Card title={t(locale, 'analyticsSessions')} icon={<IconUsers />}>
                <dl class="analytics-facts">
                  {facts.map((fact) => (
                    <div key={fact.label} class="analytics-facts__row">
                      <dt>{fact.label}</dt>
                      <dd>{fact.value}</dd>
                    </div>
                  ))}
                </dl>
              </Card>
            </div>

            <Card title={t(locale, 'topContributors')} icon={<IconTrophy />}>
              <DataTable columns={viewerColumns} data={summary.topViewers} rowKey="uniqueId" />
            </Card>
          </>
        )}
      </Page>
    );
  };
  },
);

export default AnalyticsView;
</script>
