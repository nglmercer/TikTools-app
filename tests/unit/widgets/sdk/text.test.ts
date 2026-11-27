import { expect, test } from 'vitest';
import { coreTextFields, interpolateText, isHiddenField, textDefaults } from '../../../../src/widgets/sdk/text.ts';
import { normalizeDesign } from '../../../../src/widgets/sdk/design.ts';

test('text placeholders use one pass and preserve unknown tokens', () => {
  expect(interpolateText('Thanks {{ name }} / {{name}}! {{unknown}}', { name: '{{message}}' }))
    .toBe('Thanks {{message}} / {{message}}! {{unknown}}');
  expect(interpolateText('{{constructor}}', {})).toBe('{{constructor}}');
});

test('core text fields are a per-kind subset with username and message first', () => {
  expect(coreTextFields.follow).toEqual(['handle', 'message']);
  expect(coreTextFields.gift).toEqual(['name', 'message', 'count']);
  expect(coreTextFields.chat).toEqual(['name', 'message']);
  const defaultsByKind: Record<string, Record<string, string>> = textDefaults;
  for (const [kind, fields] of Object.entries(coreTextFields)) {
    for (const field of fields) {
      expect(field in (defaultsByKind[kind] ?? {})).toBe(true);
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
