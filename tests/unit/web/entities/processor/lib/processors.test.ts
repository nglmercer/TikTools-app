import { describe, expect, it } from 'vitest';

import type { ProcessorStatusEntry, ProcessorStatusMetrics } from '../../../../../../src/shared/messages.ts';
import { sampleEventForType } from '../../../../../../src/automation/event-registry.ts';
import { processorMetricRows, processorPreviewEvent, processorStatusTone } from '../../../../../../src/web/entities/processor/lib/processors.ts';
import { EVENT_KIND } from '../../../../../shared/index.ts';
import { isRecord, propertyOf } from '../../../../support/json.ts';

const metrics: ProcessorStatusMetrics = {
  calls: 10,
  successes: 8,
  failures: 1,
  timeouts: 1,
  skippedCircuitOpen: 2,
  skippedOverloaded: 3,
  averageLatencyMs: 12,
  maxLatencyMs: 120,
};

function entry(eventTypes: string[]): ProcessorStatusEntry {
  return {
    pluginId: 'textintel',
    processorId: 'textintel.analyze',
    eventTypes,
    timeoutMs: 250,
    priority: 0,
    status: 'ready',
    metrics,
  };
}

describe('processor panel helpers', () => {
  it('status tones reuse the plugin dot vocabulary', () => {
    expect(processorStatusTone('ready')).toBe('ok');
    expect(processorStatusTone('circuit-open')).toBe('error');
    expect(processorStatusTone('disabled')).toBe('off');
    expect(processorStatusTone('unavailable')).toBe('off');
    expect(processorStatusTone('degraded')).toBe('off');
  });

  it('metric rows cover every host counter in display order', () => {
    const rows = processorMetricRows(metrics);
    expect(rows.map((row) => row.key)).toEqual([
      'calls',
      'successes',
      'failures',
      'timeouts',
      'skippedCircuitOpen',
      'skippedOverloaded',
      'averageLatencyMs',
      'maxLatencyMs',
    ]);
    expect(rows.find((row) => row.key === 'skippedOverloaded')?.value).toBe('3');
    expect(rows.find((row) => row.key === 'averageLatencyMs')?.value).toBe('12 ms');
  });

  it('preview events follow the first subscription with a chat fallback', () => {
    expect(processorPreviewEvent(entry([EVENT_KIND.tiktokGift])).type).toBe(EVENT_KIND.tiktokGift);
    const fallback = processorPreviewEvent(entry([]));
    expect(fallback.type).toBe(EVENT_KIND.tiktokChat);
    expect(typeof fallback.data).toBe('object');
    // Unknown plugin event types degrade to a minimal envelope.
    const unknown = processorPreviewEvent(entry(['plugin.unknown']));
    expect<string>(unknown.type).toBe('plugin.unknown');
    expect(unknown.id).toBe('sample-event');
  });

  it('preview events replay matching live data instead of the sample', () => {
    const live = sampleEventForType(EVENT_KIND.tiktokGift);
    if (!isRecord(live.data)) throw new Error('expected the sample gift data to be an object');
    live.data['giftName'] = 'Galaxy';
    const replay = processorPreviewEvent(entry([EVENT_KIND.tiktokGift]), live);
    expect(propertyOf(replay.data, 'giftName')).toBe('Galaxy');
    // A mismatched live event degrades to the sample of the subscribed type.
    const chat = sampleEventForType(EVENT_KIND.tiktokChat);
    const fallback = processorPreviewEvent(entry([EVENT_KIND.tiktokGift]), chat);
    expect(propertyOf(fallback.data, 'giftName')).toBe('Rose');
  });
});
