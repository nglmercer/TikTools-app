import { describe, expect, it } from 'vitest';
import type { ProcessorStatusEntry } from '../../../../../../src/shared/messages.ts';
import { filterProcessors, processorKey } from '../../../../../../src/web/widgets/plugin-processors/lib/processor-rows.ts';

function entry(pluginId: string, processorId: string, eventTypes: string[], status: ProcessorStatusEntry['status'] = 'ready') {
  return { pluginId, processorId, eventTypes, status } as ProcessorStatusEntry;
}

const entries = [entry('textintel', 'sentiment', ['tiktok.chat']), entry('deck', 'scene', [], 'circuit-open')];

describe('processor rows', () => {
  it('keys rows by plugin and processor', () => {
    expect(processorKey(entries[0]!)).toBe('textintel/sentiment');
  });

  it('filters by every word across plugin, processor, status and events', () => {
    expect(filterProcessors(entries, '  ')).toEqual(entries);
    expect(filterProcessors(entries, 'CHAT text')).toEqual([entries[0]]);
    expect(filterProcessors(entries, 'circuit')).toEqual([entries[1]]);
    expect(filterProcessors(entries, 'chat deck')).toEqual([]);
  });
});
