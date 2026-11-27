import { testMessage } from '@/shared/i18n/test-message.ts';
import { describe, expect, it } from 'vitest';
import type { EventFilter } from '../../../../../../src/automation/behavior/types.ts';
import {
  conditionFields,
  describeFilter,
  filterValues,
  isFilterMissing,
  newFilter,
  withField,
  withOperator,
  withValues,
} from '../../../../../../src/web/entities/automation/lib/conditions.ts';
import { EVENT_KIND } from '../../../../../shared/index.ts';

const t = (key: string): string => testMessage('en', key);

const filter = (patch: Partial<EventFilter>): EventFilter => ({ path: 'event.data.comment', operator: 'eq', value: '', ...patch });

describe('conditions', () => {
  it('lists localized trigger fields', () => {
    const english = conditionFields(EVENT_KIND.tiktokGift, 'en');
    const spanish = conditionFields(EVENT_KIND.tiktokGift, 'es');
    const gift = english.find((field) => field.path === 'event.data.giftName');
    expect(gift?.kind).toBe('gift');
    expect(spanish.find((field) => field.path === 'event.data.giftName')?.label).toBeTruthy();
    expect(english.every((field) => !field.path.includes('.0.'))).toBe(true);
  });

  it('starts new filters on the first field with a valid operator', () => {
    const created = newFilter(EVENT_KIND.tiktokGift);
    expect(created.path).toBe(conditionFields(EVENT_KIND.tiktokGift, 'en')[0]?.path);
    expect(created.value).toBe('');
  });

  it('keeps compatible operators when the field changes', () => {
    const next = withField(EVENT_KIND.tiktokGift, filter({ path: 'event.data.giftName', operator: 'in', values: ['Rose'] }), 'event.user.uniqueId');
    expect(next).toMatchObject({ path: 'event.user.uniqueId', operator: 'in', values: ['Rose'] });
    expect(withField(EVENT_KIND.tiktokGift, filter({ value: 'x' }), null)).toEqual({ path: '', operator: 'eq', value: '', values: undefined });
  });

  it('moves values between single and list operators', () => {
    const listed = withOperator(filter({ value: 'hi' }), 'in');
    expect(listed).toMatchObject({ operator: 'in', value: '', values: ['hi'] });
    // Every value operator keeps the list; `value` stays empty while `values` is set.
    expect(withOperator(listed, 'contains')).toMatchObject({ operator: 'contains', value: '', values: ['hi'] });
    expect(withOperator(filter({ value: 'hi' }), 'is-true')).toMatchObject({ value: '', values: undefined });
  });

  it('reports missing values unless an empty option is valid', () => {
    expect(isFilterMissing(filter({}))).toBe(true);
    expect(isFilterMissing(filter({ operator: 'is-false' }))).toBe(false);
    expect(isFilterMissing(filter({}), [{ value: '' }])).toBe(false);
    expect(isFilterMissing(filter({ operator: 'in', values: ['a'] }))).toBe(false);
  });

  it('reads and writes values for both operator shapes', () => {
    expect(filterValues(filter({ value: 'a' }))).toEqual(['a']);
    expect(withValues(filter({ operator: 'in' }), ['a', 'b'])).toMatchObject({ values: ['a', 'b'] });
    expect(withValues(filter({}), ['x', 'y'])).toMatchObject({ values: ['x', 'y'], value: '' });
  });

  it('describes a filter as a phrase', () => {
    expect(describeFilter(filter({ value: 'hi' }), 'Comment', 'is')).toBe('Comment is hi');
    expect(describeFilter(filter({ operator: 'in', values: ['a', 'b'] }), '', t('automations.operators.in'))).toBe('comment is one of a, b');
    expect(describeFilter(filter({ operator: 'is-true' }), 'Flag', t('automations.operators.is-true'))).toBe('Flag is true');
    expect(describeFilter(filter({}), 'Comment', 'is')).toBe('Comment is …');
  });
});
