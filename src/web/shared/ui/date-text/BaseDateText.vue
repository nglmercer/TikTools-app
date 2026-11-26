<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { intlLocale } from '@/shared/lib/intl.ts';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

/**
 * Responsive date display: full format on desktop, compact on narrow
 * screens, verbose detail (weekday, timezone) in the hover tooltip.
 * Values are unix seconds; `utc` pins formatting to UTC for day-based
 * values so the rendered date never shifts with the viewer's timezone.
 */
const props = withDefaults(
  defineProps<{
    value: number;
    endValue?: number;
    mode?: 'date' | 'datetime' | 'time';
    utc?: boolean;
  }>(),
  { endValue: undefined, mode: 'date', utc: false },
);

const { locale } = useI18n();

const SECS_PER_DAY = 86_400;

function zone(utc: boolean): { timeZone?: string } {
  return utc ? { timeZone: 'UTC' } : {};
}

function sameYear(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear();
}

function isToday(date: Date): boolean {
  const now = new Date();
  return date.getFullYear() === now.getFullYear() && date.getMonth() === now.getMonth() && date.getDate() === now.getDate();
}

function datePart(unixSecs: number, utc: boolean, compact: boolean): string {
  const date = new Date(unixSecs * 1_000);
  const dropYear = compact && sameYear(date, new Date());
  return date.toLocaleDateString(intlLocale(locale.value), {
    month: 'short',
    day: 'numeric',
    ...(dropYear ? {} : { year: 'numeric' }),
    ...zone(utc),
  });
}

function timePart(unixSecs: number, utc: boolean): string {
  return new Date(unixSecs * 1_000).toLocaleTimeString(intlLocale(locale.value), {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    ...zone(utc),
  });
}

function format(unixSecs: number, compact: boolean): string {
  if (props.mode === 'time') return timePart(unixSecs, props.utc);
  const date = new Date(unixSecs * 1_000);
  if (props.mode === 'datetime') {
    // Datetimes are instants: compact shows just the clock when the day is today.
    if (compact && !props.utc && isToday(date)) return timePart(unixSecs, props.utc);
    return `${datePart(unixSecs, props.utc, compact)} · ${timePart(unixSecs, props.utc)}`;
  }
  return datePart(unixSecs, props.utc, compact);
}

function formatSpanValue(compact: boolean): string {
  const start = format(props.value, compact);
  if (!isSpan.value) return start;
  return `${start} – ${format(props.endValue as number, compact)}`;
}

function verbose(unixSecs: number): string {
  const date = new Date(unixSecs * 1_000);
  const day = date.toLocaleDateString(intlLocale(locale.value), {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    ...zone(props.utc),
  });
  if (props.mode === 'date') return day;
  if (props.mode === 'time') return timePart(unixSecs, props.utc);
  const zoneName = props.utc
    ? 'UTC'
    : new Intl.DateTimeFormat(intlLocale(locale.value), { timeZoneName: 'short' })
        .formatToParts(date)
        .find((part) => part.type === 'timeZoneName')?.value ?? '';
  return `${day} · ${timePart(unixSecs, props.utc)}${zoneName ? ` ${zoneName}` : ''}`;
}

const full = computed(() => formatSpanValue(false));
const compact = computed(() => formatSpanValue(true));
const detail = computed(() =>
  isSpan.value ? `${verbose(props.value)} – ${verbose(props.endValue as number)}` : verbose(props.value),
);
const singleDay = computed(() => props.endValue === undefined || Math.floor(props.endValue / SECS_PER_DAY) === Math.floor(props.value / SECS_PER_DAY));
const isSpan = computed(() => !singleDay.value);
</script>

<template>
  <time
    v-if="singleDay"
    v-tooltip="detail"
    class="base-date-text"
    :datetime="new Date(value * 1_000).toISOString()"
  >
    <span class="base-date-text__full">{{ full }}</span>
    <span class="base-date-text__compact">{{ compact }}</span>
  </time>
  <span
    v-else
    v-tooltip="detail"
    class="base-date-text"
  >
    <span class="base-date-text__full">{{ full }}</span>
    <span class="base-date-text__compact">{{ compact }}</span>
  </span>
</template>

<style scoped>
.base-date-text {
  font-variant-numeric: tabular-nums;
}

.base-date-text__compact {
  display: none;
}

@media (width < 600px) {
  .base-date-text__full {
    display: none;
  }

  .base-date-text__compact {
    display: inline;
  }
}
</style>
