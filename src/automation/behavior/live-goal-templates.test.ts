import { expect, test } from 'bun:test';
import { liveGoalTemplates } from './live-goal-templates.ts';
import { applyConditionTemplate } from './condition-templates.ts';
import { matchesFilter } from './filters.ts';
import type { AutomationEvent } from '../types.ts';
test('like goal crosses 15 once even when a packet skips the exact target', () => {
  for (const template of liveGoalTemplates()) {
    const filters = applyConditionTemplate(template, { goal: '15' });
    const matches = (previous: number, total: number) => filters.every((filter) => matchesFilter(filter, {
      type: 'tiktok.like', data: { aggregation: { global: { previous, total }, user: { previous, total } } },
    } as unknown as AutomationEvent));
    expect(matches(0, 10)).toBe(false);
    expect(matches(10, 20)).toBe(true);
    expect(matches(20, 30)).toBe(false);
    expect(matches(14, 15)).toBe(true);
    expect(matches(15, 15)).toBe(false);
  }
});
