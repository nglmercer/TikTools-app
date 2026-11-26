import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import type { AnalyticsSummaryData } from '../../../../shared/messages.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { fetchAnalyticsSummary } from '../api/analytics-api.ts';
import { tzOffsetSecsForDay, type AnalyticsDaySpan } from '../lib/analytics-range.ts';

export const useAnalyticsStore = defineStore('analytics', () => {
  const client = useControlClient();
  const summary = shallowRef<AnalyticsSummaryData | null>(null);
  /** Same-length span immediately before the selected one, for deltas and the comparison series. */
  const previous = shallowRef<AnalyticsSummaryData | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  let latest = 0;

  async function load(creator: string, span: AnalyticsDaySpan): Promise<void> {
    const request = ++latest;
    loading.value = true;
    error.value = null;
    const creatorUniqueId = creator.trim().replace(/^@/, '');
    const length = span.endDay - span.startDay + 1;
    const tzOffsetSecs = tzOffsetSecsForDay(span.endDay);
    try {
      const [result, previousResult] = await Promise.all([
        fetchAnalyticsSummary(client, { creatorUniqueId, startDay: span.startDay, endDay: span.endDay, tzOffsetSecs }),
        fetchAnalyticsSummary(client, { creatorUniqueId, startDay: span.startDay - length, endDay: span.endDay - length, tzOffsetSecs }),
      ]);
      if (request === latest) {
        summary.value = result;
        previous.value = previousResult;
      }
    } catch (failure) {
      if (request === latest) error.value = errorMessage(failure);
    } finally {
      if (request === latest) loading.value = false;
    }
  }

  return { summary, previous, loading, error, load };
});
