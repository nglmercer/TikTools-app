import type { AnalyticsSummaryData } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';

export interface AnalyticsSummaryRequest {
  creatorUniqueId: string;
  startDay: number;
  endDay: number;
  tzOffsetSecs: number;
  limit?: number;
}

export async function fetchAnalyticsSummary(
  client: ControlClient,
  request: AnalyticsSummaryRequest,
): Promise<AnalyticsSummaryData | null> {
  const result = await client.call<{ summary: AnalyticsSummaryData | null }>('analytics.summary', {
    limit: 100,
    ...request,
  });
  return result.summary;
}
