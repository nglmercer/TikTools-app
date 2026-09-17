<script lang="tsx">
import { defineVueFunctional } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import { Chip, ChipGroup } from '../ui/Card.vue';
import { DatePicker } from '../ui/DatePicker.vue';
import { ANALYTICS_RANGE_KEYS, type AnalyticsRangeKey } from './analytics-range.ts';

type AnalyticsRangeFilterProps = {
  locale: Locale;
  range: AnalyticsRangeKey;
  customStart: string;
  customEnd: string;
  onRangeChange: (range: AnalyticsRangeKey) => void;
  onCustomStartChange: (next: string) => void;
  onCustomEndChange: (next: string) => void;
};

const RANGE_LABELS: Record<AnalyticsRangeKey, 'rangeToday' | 'range7Days' | 'range30Days' | 'range90Days' | 'rangeCustom'> = {
  today: 'rangeToday',
  '7d': 'range7Days',
  '30d': 'range30Days',
  '90d': 'range90Days',
  custom: 'rangeCustom',
};

/** Reuses Chip/ChipGroup + DatePicker instead of bespoke toolbar buttons. */
export const AnalyticsRangeFilter = defineVueFunctional<AnalyticsRangeFilterProps>((props) => {
  const { locale, range, customStart, customEnd, onRangeChange, onCustomStartChange, onCustomEndChange } = props;
  return (
    <div class="analytics-toolbar">
      <ChipGroup>
        {ANALYTICS_RANGE_KEYS.map((key) => (
          <Chip key={key} active={range === key} onClick={() => onRangeChange(key)}>
            {t(locale, RANGE_LABELS[key])}
          </Chip>
        ))}
      </ChipGroup>
      {range === 'custom' ? (
        <div class="analytics-custom">
          <DatePicker
            name="analytics-start"
            label={t(locale, 'rangeFrom')}
            value={customStart}
            onValueChange={onCustomStartChange}
            max={customEnd}
          />
          <DatePicker
            name="analytics-end"
            label={t(locale, 'rangeTo')}
            value={customEnd}
            onValueChange={onCustomEndChange}
            min={customStart}
          />
        </div>
      ) : null}
    </div>
  );
});

export default AnalyticsRangeFilter;
</script>
