import { ref } from 'vue';

import type { AnalyticsSummaryData } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';

export interface AnalyticsSummaryResult {
  summary: AnalyticsSummaryData | null;
}

/** Analytics summary for the active creator. */
export function useAnalytics(control: ControlClient, activeCreator: () => string) {
  const analyticsSummary = ref<AnalyticsSummaryData | null>(null);

  const handleGetAnalyticsRange = (startDay: number, endDay: number, tzOffsetSecs = 0): void => {
    const creatorUniqueId = activeCreator().trim().replace(/^@/, '');
    void control
      .call<AnalyticsSummaryResult>('analytics.summary', {
        creatorUniqueId,
        startDay,
        endDay,
        limit: 10,
        tzOffsetSecs,
      })
      .then((result) => {
        analyticsSummary.value = result.summary;
      })
      .catch((failure: unknown) => {
        console.warn(`analytics.summary failed: ${errorMessage(failure)}`);
      });
  };

  return {
    analyticsSummary,
    handleGetAnalyticsRange,
  };
}
