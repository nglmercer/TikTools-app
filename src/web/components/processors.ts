import type { AutomationEvent } from '../../automation/types.ts';
import { sampleEventForType } from '../../automation/event-registry.ts';
import type {
  ProcessorStatusEntry,
  ProcessorStatusMetrics,
} from '../../shared/messages.ts';

/** Dot tone for a processor status, reusing the plugins stylesheet tones. */
export type ProcessorStatusTone = 'is-ok' | 'is-err' | 'is-off';

export function processorStatusTone(status: ProcessorStatusEntry['status']): ProcessorStatusTone {
  if (status === 'ready') return 'is-ok';
  if (status === 'circuit-open') return 'is-err';
  return 'is-off';
}

export type ProcessorMetricRow = { key: string; value: string };

/** Stable metric rows in display order; labels come from i18n copy by key. */
export function processorMetricRows(metrics: ProcessorStatusMetrics): ProcessorMetricRow[] {
  return [
    { key: 'calls', value: `${metrics.calls}` },
    { key: 'successes', value: `${metrics.successes}` },
    { key: 'failures', value: `${metrics.failures}` },
    { key: 'timeouts', value: `${metrics.timeouts}` },
    { key: 'skippedCircuitOpen', value: `${metrics.skippedCircuitOpen}` },
    { key: 'skippedOverloaded', value: `${metrics.skippedOverloaded}` },
    { key: 'averageLatencyMs', value: `${metrics.averageLatencyMs} ms` },
    { key: 'maxLatencyMs', value: `${metrics.maxLatencyMs} ms` },
  ];
}

/**
 * Sample live event for previewing one processor: the registry sample for
 * its first subscribed type, else the chat sample. Unknown types degrade to
 * a minimal envelope instead of failing.
 */
export function processorPreviewEvent(entry: ProcessorStatusEntry): AutomationEvent {
  const [first] = entry.eventTypes;
  return sampleEventForType(first ?? 'tiktok.chat');
}
