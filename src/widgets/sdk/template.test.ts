import { expect, test } from 'bun:test';
import { styleVariables } from './template.ts';

test('editor styling rejects CSS injection and bounds geometry', () => {
  expect(styleVariables({ background: '#112233', textColor: '#ffffffff', accent: 'url(https://example.com)', radius: 100 })).toEqual({
    '--widget-background': '#112233', '--widget-textColor': '#ffffffff', '--widget-radius': '48px',
  });
  expect(styleVariables({ radius: NaN })).toEqual({});
});
