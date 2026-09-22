import { expect, test } from 'bun:test';
import { interpolateText } from './text.ts';
import { normalizeDesign } from './design.ts';

test('text placeholders use one pass and preserve unknown tokens', () => {
  expect(interpolateText('Thanks {{ name }} / {{name}}! {{unknown}}', { name: '{{message}}' }))
    .toBe('Thanks {{message}} / {{message}}! {{unknown}}');
  expect(interpolateText('{{constructor}}', {})).toBe('{{constructor}}');
});

test('saved text preserves intentionally hidden lines and limits fields', () => {
  expect(normalizeDesign({ text: { title: '', name: '{{name}}', message: 'x'.repeat(400), evil: 'no', count: 5 } }).text)
    .toEqual({ title: '', name: '{{name}}', message: 'x'.repeat(300) });
});
