import { expect, test } from 'bun:test';
import { coreTextFields, interpolateText, isHiddenField, textDefaults } from './text.ts';
import { normalizeDesign } from './design.ts';

test('text placeholders use one pass and preserve unknown tokens', () => {
  expect(interpolateText('Thanks {{ name }} / {{name}}! {{unknown}}', { name: '{{message}}' }))
    .toBe('Thanks {{message}} / {{message}}! {{unknown}}');
  expect(interpolateText('{{constructor}}', {})).toBe('{{constructor}}');
});

test('core text fields are a per-kind subset with username and message first', () => {
  expect(coreTextFields.follow).toEqual(['handle', 'message']);
  expect(coreTextFields.gift).toEqual(['name', 'message', 'count']);
  expect(coreTextFields.chat).toEqual(['name', 'message']);
  for (const kind of Object.keys(textDefaults) as Array<keyof typeof textDefaults>) {
    for (const field of coreTextFields[kind]) {
      expect(field in textDefaults[kind]).toBe(true);
    }
  }
});

test('hidden fields resolve from the design list only', () => {
  expect(isHiddenField(undefined, 'title')).toBe(false);
  expect(isHiddenField({}, 'title')).toBe(false);
  expect(isHiddenField({ hiddenText: ['title', 'name'] }, 'title')).toBe(true);
  expect(isHiddenField({ hiddenText: ['title', 'name'] }, 'message')).toBe(false);
});

test('saved text preserves intentionally hidden lines and limits fields', () => {
  expect(normalizeDesign({ text: { title: '', name: '{{name}}', message: 'x'.repeat(400), evil: 'no', count: 5 } }).text)
    .toEqual({ title: '', name: '{{name}}', message: 'x'.repeat(300) });
});
