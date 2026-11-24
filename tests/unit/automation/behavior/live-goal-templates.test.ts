import { expect, test } from 'vitest';
import { liveGoalTemplates } from '../../../../src/automation/behavior/live-goal-templates.ts';
import { applyConditionTemplate } from '../../../../src/automation/behavior/condition-templates.ts';
import { matchesFilter } from '../../../../src/automation/behavior/testing/filter-matcher.ts';
import type { AutomationEvent } from '../../../../src/automation/types.ts';
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
