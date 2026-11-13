import { expect, it } from 'vitest';
import { formatDate, intlLocale } from './intl.ts';

it('uses Chinese and Japanese date conventions instead of the English fallback', () => {
  const at = new Date(2026, 0, 2, 12).valueOf();
  expect(intlLocale('zh')).toBe('zh-CN');
  expect(intlLocale('jp')).toBe('ja-JP');
  expect(formatDate(at, 'zh')).toMatch(/2026年1月2日/);
  expect(formatDate(at, 'ja')).toMatch(/2026年1月2日/);
});
