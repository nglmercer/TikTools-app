import { describe, expect, test } from 'bun:test';
import type { AutomationEvent } from '../types.ts';
import { matchesFilter } from './filters.ts';
import { applyConditionTemplate } from './condition-templates.ts';
import { liveGoalTemplates, recurringLikeGoalTemplates } from './live-goal-templates.ts';
import { normalizeEvent } from './schema.ts';
import { parseRuleTemplate } from '../../web/entities/rule-template/lib/rule-templates.ts';

function likeBatch(previous: number, total: number, count = total - previous): AutomationEvent {
  return {
    id: `fake-like-${total}`,
    type: 'tiktok.like',
    timestamp: total,
    data: {
      count,
      // TikTok room snapshot must never be added to observed batch deltas.
      total: 50_000 + total,
      aggregation: { global: { previous, total }, user: { previous, total } },
    },
  } as unknown as AutomationEvent;
}

describe('backwards-compatible like goal templates', () => {
  test('keeps old preset IDs and one-time threshold filters untouched', () => {
    const old = liveGoalTemplates();
    expect(old.map((template) => template.id)).toEqual(['likes-goal-global', 'likes-goal-user']);
    const filters = applyConditionTemplate(old[0]!, { goal: '150' });
    expect(filters).toEqual([
      { path: 'event.data.aggregation.global.previous', operator: 'lt', value: '150' },
      { path: 'event.data.aggregation.global.total', operator: 'gte', value: '150' },
    ]);
    expect(filters.every((filter) => matchesFilter(filter, likeBatch(105, 150)))).toBe(true);
    expect(filters.every((filter) => matchesFilter(filter, likeBatch(150, 300, 150)))).toBe(false);
    // Existing packet-count templates still see the original per-packet count.
    expect(matchesFilter({ path: 'event.data.count', operator: 'gte', value: '150' }, likeBatch(150, 450, 300))).toBe(true);
  });

  test('offers a new opt-in recurring 150-like goal and preserves it on edit', () => {
    const recurring = recurringLikeGoalTemplates()[0]!;
    const filter = applyConditionTemplate(recurring, { goal: '150' })[0]!;
    expect(filter).toEqual({
      path: 'event.data.aggregation.global',
      operator: 'crosses-multiple',
      value: '150',
    });
    expect(matchesFilter(filter!, likeBatch(105, 149))).toBe(false);
    expect(matchesFilter(filter!, likeBatch(149, 150))).toBe(true);
    expect(matchesFilter(filter!, likeBatch(150, 450, 300))).toBe(true);
    const normalized = normalizeEvent({
      schemaVersion: 1, id: 'like-150', name: 'Every 150',
      enabled: true, trigger: 'tiktok.like', filters: [filter],
      cooldownMs: 0, cooldownScope: 'global', actionIds: [], runMode: 'all',
    });
    expect(normalized.filters).toEqual([filter]);
    expect(matchesFilter({ ...filter!, value: '0' }, likeBatch(0, 150))).toBe(false);
    expect(matchesFilter({ ...filter!, value: '-1' }, likeBatch(0, 150))).toBe(false);
  });

  test('accepts recurring templates in the existing web importer', () => {
    const result = parseRuleTemplate({
      templateVersion: 1, id: 'like-150-e2e', title: 'Every 150 likes',
      actions: [{ name: 'Log likes', typeId: 'core.log', config: { message: '150 likes' } }],
      event: {
        name: 'Every 150 likes', trigger: 'tiktok.like',
        filters: [{ path: 'event.data.aggregation.global', operator: 'crosses-multiple', value: '150' }],
      },
    });
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.template.event.filters[0]?.operator).toBe('crosses-multiple');
  });
});
