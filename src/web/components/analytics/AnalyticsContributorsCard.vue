<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import type { AnalyticsTopViewer } from '../../../shared/messages.ts';
import { IconTrophy } from '../icons.vue';
import { Card } from '../ui/Card.vue';
import { DataTable, type Column } from '../ui/Table.vue';
import { formatTime } from './analytics-intraday.ts';
import { formatCount } from './analytics-range.ts';

type AnalyticsContributorsCardProps = {
  locale: Locale;
  topViewers: AnalyticsTopViewer[];
};

/** Top contributors table. Reuses Card + DataTable instead of bespoke markup. */
export const AnalyticsContributorsCard = defineVueFunctional<AnalyticsContributorsCardProps>((props) => {
  const { locale, topViewers } = props;

  const columns: Column<AnalyticsTopViewer>[] = [
    {
      key: 'rank',
      header: t(locale, 'rank'),
      width: '64px',
      render: (_row, idx) =>
        idx < 3 ? (
          <span
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 4,
              fontWeight: 800,
              color: idx === 0 ? '#b45309' : idx === 1 ? '#64748b' : '#92400e',
            }}
          >
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
    {
      key: 'lastSeen',
      header: t(locale, 'lastActive'),
      width: '90px',
      align: 'right' as const,
      render: (row) => (
        <span style={{ color: 'var(--text-muted)', fontSize: 12, fontVariantNumeric: 'tabular-nums' }}>
          {formatTime(row.lastSeen, locale)}
        </span>
      ),
    },
  ];

  return (
    <Card title={t(locale, 'topContributors')} icon={<IconTrophy />}>
      <DataTable columns={columns} data={topViewers} rowKey="uniqueId" />
    </Card>
  );
});

export default AnalyticsContributorsCard;
</script>
