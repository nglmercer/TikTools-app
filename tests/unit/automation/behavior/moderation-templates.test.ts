import { afterEach, describe, expect, test } from 'vitest';

import {
  applyConditionTemplate,
  clearConditionTemplates,
  conditionTemplatesFor,
  type ConditionTemplate,
} from '../../../../src/automation/behavior/condition-templates.ts';
import { matchesFilter } from '../../../../src/automation/behavior/testing/filter-matcher.ts';
import { moderationConditionTemplates, registerModerationTemplates } from '../../../../src/automation/behavior/moderation-templates.ts';
import { EVENT_KIND } from '../../../shared/index.ts';
import { automationEvent, commentIntel } from '../../support/automation-event.ts';

afterEach(() => {
  clearConditionTemplates();
});

describe('moderation condition templates', () => {
  test('low/medium/high target chat with descending thresholds', () => {
    registerModerationTemplates();
    registerModerationTemplates();
    const found = conditionTemplatesFor(EVENT_KIND.tiktokChat);
    expect(found.map((template) => template.id)).toEqual([
      'moderation-low',
      'moderation-medium',
      'moderation-high',
    ]);
    expect(conditionTemplatesFor(EVENT_KIND.tiktokGift)).toEqual([]);
    expect(found.map((template) => template.params?.[0]?.default)).toEqual(['0.9', '0.7', '0.5']);
    for (const template of found) {
      // Every row references a declared param or a literal: the config
      // modal can always resolve what it shows.
      const keys = new Set((template.params ?? []).map((param) => param.key));
      for (const row of template.rows) {
        if (typeof row.value !== 'string') expect(keys.has(row.value.param)).toBe(true);
      }
    }
  });

  test('high bundles the spam flag with its threshold', () => {
    const high = moderationConditionTemplates().find((template) => template.id === 'moderation-high');
    expect(high?.rows).toHaveLength(2);
    expect(high?.rows[1]).toEqual({ path: 'event.intel.comment.spam.detected', operator: 'is-true', value: '' });
  });

  test('applied templates match through the real filter engine', () => {
    const event = automationEvent({
      type: EVENT_KIND.tiktokChat,
      data: {},
      intel: commentIntel({ composition: { repetitionScore: 0.8 }, spam: { detected: false } }),
    });
    const mustFind = (id: string): ConditionTemplate => {
      const found = moderationConditionTemplates().find((template) => template.id === id);
      if (!found) throw new Error(`missing template ${id}`);
      return found;
    };
    const matches = (id: string, answers: Record<string, string>): boolean =>
      applyConditionTemplate(mustFind(id), answers).every((filter) => matchesFilter(filter, event));
    // 0.8 clears medium (0.7) but not low (0.9); high fails on the spam flag.
    expect(matches('moderation-low', {})).toBe(false);
    expect(matches('moderation-medium', {})).toBe(true);
    expect(matches('moderation-high', {})).toBe(false);
    // A custom answer moves the line: 0.75 still clears medium's row.
    expect(matches('moderation-medium', { threshold: '0.75' })).toBe(true);
    expect(matches('moderation-medium', { threshold: '0.85' })).toBe(false);
  });
});
