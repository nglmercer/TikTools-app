import { afterEach, describe, expect, test } from 'bun:test';

import {
  applyConditionTemplate,
  clearConditionTemplates,
  conditionTemplatesFor,
  registerConditionTemplates,
  type ConditionTemplate,
} from './condition-templates.ts';

afterEach(() => {
  clearConditionTemplates();
});

function template(overrides: Partial<ConditionTemplate> = {}): ConditionTemplate {
  return {
    id: 'demo.basic',
    title: { default: 'Demo', i18key: '' },
    rows: [{ path: 'event.data.comment', operator: 'contains', value: 'hi' }],
    ...overrides,
  };
}

describe('condition template registry', () => {
  test('registers and filters templates per trigger', () => {
    registerConditionTemplates([
      template({ id: 'chat.only', triggers: ['tiktok.chat'] }),
      template({ id: 'any.where' }),
    ]);
    expect(conditionTemplatesFor('tiktok.chat').map((entry) => entry.id).sort())
      .toEqual(['any.where', 'chat.only']);
    expect(conditionTemplatesFor('tiktok.gift').map((entry) => entry.id)).toEqual(['any.where']);
  });

  test('re-registering an id replaces the template', () => {
    registerConditionTemplates([template({ rows: [{ path: 'event.data.a', operator: 'eq', value: '1' }] })]);
    registerConditionTemplates([template({ rows: [{ path: 'event.data.b', operator: 'eq', value: '2' }] })]);
    const found = conditionTemplatesFor('tiktok.chat');
    expect(found).toHaveLength(1);
    expect(found[0]?.rows[0]?.path).toBe('event.data.b');
  });

  test('entries without id, rows, or paths are skipped', () => {
    registerConditionTemplates([
      template({ id: '  ' }),
      template({ id: 'norows', rows: [] }),
      template({ id: 'badrows', rows: [{ path: '  ', operator: 'eq', value: 'x' }] }),
      template({ id: 'ok' }),
    ]);
    expect(conditionTemplatesFor('tiktok.chat').map((entry) => entry.id)).toEqual(['ok']);
  });

  test('clear drops everything', () => {
    registerConditionTemplates([template()]);
    clearConditionTemplates();
    expect(conditionTemplatesFor('tiktok.chat')).toEqual([]);
  });
});

describe('applyConditionTemplate', () => {
  test('literals pass through with values carried over', () => {
    const filters = applyConditionTemplate(
      template({ rows: [{ path: 'event.data.comment', operator: 'in', value: '', values: ['a', 'b'] }] }),
      {},
    );
    expect(filters).toEqual([{ path: 'event.data.comment', operator: 'in', value: '', values: ['a', 'b'] }]);
  });

  test('params substitute answers, normalize numbers, and fall back to defaults', () => {
    const tpl = template({
      params: [
        { key: 'threshold', label: { default: 'Threshold', i18key: '' }, kind: 'number', default: '0.7' },
        { key: 'word', label: { default: 'Word', i18key: '' }, kind: 'text', default: 'spam' },
      ],
      rows: [
        { path: 'event.intel.comment.spam.score', operator: 'gte', value: { param: 'threshold' } },
        { path: 'event.data.comment', operator: 'contains', value: { param: 'word' } },
      ],
    });
    expect(applyConditionTemplate(tpl, { threshold: '0.90', word: ' free ' }).map((filter) => filter.value))
      .toEqual(['0.9', 'free']);
    // Missing answers use defaults; garbage numbers fall back too.
    expect(applyConditionTemplate(tpl, {}).map((filter) => filter.value)).toEqual(['0.7', 'spam']);
    expect(applyConditionTemplate(tpl, { threshold: 'lots' })[0]?.value).toBe('0.7');
  });

  test('unknown param keys resolve leniently instead of throwing', () => {
    const filters = applyConditionTemplate(
      template({ rows: [{ path: 'event.data.comment', operator: 'eq', value: { param: 'ghost' } }] }),
      {},
    );
    expect(filters).toEqual([{ path: 'event.data.comment', operator: 'eq', value: '' }]);
  });
});
