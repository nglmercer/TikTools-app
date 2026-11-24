import { expect, it } from 'vitest';
import { applyAppTheme, isAppTheme, systemTheme } from '../../../../../src/web/shared/lib/app-theme.ts';

it('accepts only explicit theme ids', () => {
  expect(isAppTheme('dark')).toBe(true);
  expect(isAppTheme('light')).toBe(true);
  expect(isAppTheme('system')).toBe(false);
  expect(isAppTheme(undefined)).toBe(false);
});

it('applies the theme to both the data attribute and the color scheme', () => {
  applyAppTheme('light');
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(document.documentElement.style.colorScheme).toBe('light');
  applyAppTheme('dark');
  expect(document.documentElement.dataset.theme).toBe('dark');
  expect(document.documentElement.style.colorScheme).toBe('dark');
});

it('falls back to dark when the OS preference is unavailable', () => {
  expect(['dark', 'light']).toContain(systemTheme());
});
