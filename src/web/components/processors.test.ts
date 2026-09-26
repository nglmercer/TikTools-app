import { describe, expect, test } from 'bun:test';

import type { ProcessorStatusEntry, ProcessorStatusMetrics } from '../../shared/messages.ts';
import { sampleEventForType } from '../../automation/event-registry.ts';
import { processorMetricRows, processorPreviewEvent, processorStatusTone } from './processors.ts';

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
  test('status tones reuse the plugin dot vocabulary', () => {
    expect(processorStatusTone('ready')).toBe('is-ok');
    expect(processorStatusTone('circuit-open')).toBe('is-err');
    expect(processorStatusTone('disabled')).toBe('is-off');
    expect(processorStatusTone('unavailable')).toBe('is-off');
    expect(processorStatusTone('degraded')).toBe('is-off');
  });

  test('metric rows cover every host counter in display order', () => {
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

  test('preview events follow the first subscription with a chat fallback', () => {
    expect(processorPreviewEvent(entry(['tiktok.gift'])).type).toBe('tiktok.gift');
    const fallback = processorPreviewEvent(entry([]));
    expect(fallback.type).toBe('tiktok.chat');
    expect(typeof fallback.data).toBe('object');
    // Unknown plugin event types degrade to a minimal envelope.
    const unknown = processorPreviewEvent(entry(['plugin.unknown']));
    expect(unknown.type as string).toBe('plugin.unknown');
    expect(unknown.id).toBe('sample-event');
  });

  test('preview events replay matching live data instead of the sample', () => {
    const live = sampleEventForType('tiktok.gift');
    (live.data as Record<string, unknown>)['giftName'] = 'Galaxy';
    const replay = processorPreviewEvent(entry(['tiktok.gift']), live);
    expect((replay.data as Record<string, unknown>)['giftName']).toBe('Galaxy');
    // A mismatched live event degrades to the sample of the subscribed type.
    const chat = sampleEventForType('tiktok.chat');
    const fallback = processorPreviewEvent(entry(['tiktok.gift']), chat);
    expect((fallback.data as Record<string, unknown>)['giftName']).toBe('Rose');
  });
});
