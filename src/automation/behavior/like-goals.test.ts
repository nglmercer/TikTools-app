import { describe, expect, test } from 'bun:test';
import type { AutomationEvent } from '../types.ts';
import { matchesFilter } from './filters.ts';
import { applyConditionTemplate } from './condition-templates.ts';
import { liveGoalTemplates } from './live-goal-templates.ts';
import { normalizeEvent } from './schema.ts';
import { parseRuleTemplate } from '../../web/entities/rule-template/lib/rule-templates.ts';

function likeBatch(previous: number, total: number, count = total - previous): AutomationEvent {
  return {
    id: `simulated-like-${total}`,
    type: 'tiktok.like',
    timestamp: total,
    data: {
      count,
      // A TikTok room snapshot is not an increment to the observed counter.
      total: 50_000 + total,
      aggregation: { global: { previous, total }, user: { previous, total } },
    },
  } as unknown as AutomationEvent;
}

describe('legacy like goal template compatibility', () => {
  test('preserves both preset IDs and the existing threshold operators', () => {
    const templates = liveGoalTemplates();
    expect(templates.map((template) => template.id)).toEqual(['likes-goal-global', 'likes-goal-user']);
    expect(applyConditionTemplate(templates[0]!, { goal: '150' })).toEqual([
      { path: 'event.data.aggregation.global.previous', operator: 'lt', value: '150' },
      { path: 'event.data.aggregation.global.total', operator: 'gte', value: '150' },
    ]);
    expect(applyConditionTemplate(templates[1]!, { goal: '150' })).toEqual([
      { path: 'event.data.aggregation.user.previous', operator: 'lt', value: '150' },
      { path: 'event.data.aggregation.user.total', operator: 'gte', value: '150' },
    ]);
  });

  test('45 + 60 + 45 + 300 fires each 150-like goal only at the first crossing', () => {
    const [global, viewer] = liveGoalTemplates();
    const globalFilters = applyConditionTemplate(global!, { goal: '150' });
    const userFilters = applyConditionTemplate(viewer!, { goal: '150' });
    let previous = 0;
    const globalHits: number[] = [];
    const userHits: number[] = [];
    const packetHits: number[] = [];
    for (const count of [45, 60, 45, 300]) {
      const total = previous + count;
      const event = likeBatch(previous, total, count);
      if (globalFilters.every((filter) => matchesFilter(filter, event))) globalHits.push(total);
      if (userFilters.every((filter) => matchesFilter(filter, event))) userHits.push(total);
      if (matchesFilter({ path: 'event.data.count', operator: 'gte', value: '150' }, event)) {
        packetHits.push(count);
      }
      previous = total;
    }
    expect(globalHits).toEqual([150]);
    expect(userHits).toEqual([150]);
    expect(packetHits).toEqual([300]);
  });

  test('web importer and normalization preserve existing operators', () => {
    const filters = applyConditionTemplate(liveGoalTemplates()[0]!, { goal: '150' });
    const normalized = normalizeEvent({
      schemaVersion: 1, id: 'like-150', name: 'First 150',
      enabled: true, trigger: 'tiktok.like', filters,
      cooldownMs: 0, cooldownScope: 'global', actionIds: [], runMode: 'all',
    });
    expect(normalized.filters).toEqual(filters);
    const result = parseRuleTemplate({
      templateVersion: 1, id: 'like-150', title: 'First 150 likes',
      actions: [{ name: 'Log likes', typeId: 'core.log', config: { message: '150 likes' } }],
      event: { name: 'First 150', trigger: 'tiktok.like', filters },
    });
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.template.event.filters).toEqual(filters);
  });
});
