import { describe, expect, test } from 'vitest';
import type { AutomationEvent } from '../../../../src/automation/types.ts';
import { matchesFilter } from '../../../../src/automation/behavior/testing/filter-matcher.ts';
import { applyConditionTemplate } from '../../../../src/automation/behavior/condition-templates.ts';
import { liveGoalTemplates } from '../../../../src/automation/behavior/live-goal-templates.ts';
import { normalizeEvent } from '../../../../src/automation/behavior/schema.ts';
import { parseRuleTemplate } from '../../../../src/web/entities/rule-template/lib/rule-templates.ts';
import { ACTION_TYPE, EVENT_KIND, defined } from '../../../shared/index.ts';
import { automationEvent } from '../../support/automation-event.ts';

function likeBatch(previous: number, total: number, count = total - previous): AutomationEvent {
  return automationEvent({
    id: `simulated-like-${total}`,
    type: EVENT_KIND.tiktokLike,
    timestamp: total,
    data: {
      count,
      // A TikTok room snapshot is not an increment to the observed counter.
      total: 50_000 + total,
      aggregation: { global: { previous, total }, user: { previous, total } },
    },
  });
}

describe('legacy like goal template compatibility', () => {
  test('preserves both preset IDs and the existing threshold operators', () => {
    const templates = liveGoalTemplates();
    expect(templates.map((template) => template.id)).toEqual(['likes-goal-global', 'likes-goal-user']);
    expect(applyConditionTemplate(defined(templates[0]), { goal: '150' })).toEqual([
      { path: 'event.data.aggregation.global.previous', operator: 'lt', value: '150' },
      { path: 'event.data.aggregation.global.total', operator: 'gte', value: '150' },
    ]);
    expect(applyConditionTemplate(defined(templates[1]), { goal: '150' })).toEqual([
      { path: 'event.data.aggregation.user.previous', operator: 'lt', value: '150' },
      { path: 'event.data.aggregation.user.total', operator: 'gte', value: '150' },
    ]);
  });

  test('45 + 60 + 45 + 300 fires each 150-like goal only at the first crossing', () => {
    const [global, viewer] = liveGoalTemplates();
    const globalFilters = applyConditionTemplate(defined(global), { goal: '150' });
    const userFilters = applyConditionTemplate(defined(viewer), { goal: '150' });
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
    const filters = applyConditionTemplate(defined(liveGoalTemplates()[0]), { goal: '150' });
    const normalized = normalizeEvent({
      schemaVersion: 1, id: 'like-150', name: 'First 150',
      enabled: true, trigger: EVENT_KIND.tiktokLike, filters,
      cooldownMs: 0, cooldownScope: 'global', actionIds: [], runMode: 'all',
    });
    expect(normalized.filters).toEqual(filters);
    const result = parseRuleTemplate({
      templateVersion: 1, id: 'like-150', title: 'First 150 likes',
      actions: [{ name: 'Log likes', typeId: ACTION_TYPE.coreLog, config: { message: '150 likes' } }],
      event: { name: 'First 150', trigger: EVENT_KIND.tiktokLike, filters },
    });
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.template.event.filters).toEqual(filters);
  });
});
