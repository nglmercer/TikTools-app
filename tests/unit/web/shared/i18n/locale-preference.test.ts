import { afterEach, describe, expect, it } from 'vitest';
import { detectLocale, storedLocale, storeLocale } from '../../../../../src/web/shared/i18n/locale-preference.ts';
import { STORAGE_KEY } from '../../../../shared/index.ts';

afterEach(() => {
  localStorage.clear();
});

describe('locale preference', () => {
  it('prefers the stored locale', () => {
    storeLocale('pt');
    expect(storedLocale()).toBe('pt');
    expect(detectLocale(['en-US'])).toBe('pt');
  });

  it('ignores unsupported stored values', () => {
    localStorage.setItem(STORAGE_KEY.locale, 'de');
    expect(storedLocale()).toBeNull();
  });

  it('matches the first supported system language', () => {
    expect(detectLocale(['de-DE', 'fr-FR', 'es-MX'])).toBe('fr');
    expect(detectLocale(['pt-BR'])).toBe('pt');
    expect(detectLocale(['de-DE'])).toBe('en');
  });

  it('detects CJK system locales and canonicalizes the jp alias', () => {
    expect(detectLocale(['zh-CN', 'en-US'])).toBe('zh');
    expect(detectLocale(['ja-JP', 'en-US'])).toBe('ja');
    localStorage.setItem(STORAGE_KEY.locale, 'jp');
    expect(storedLocale()).toBe('ja');
    expect(localStorage.getItem(STORAGE_KEY.locale)).toBe('ja');
    expect(detectLocale(['en-US'])).toBe('ja');
  });
});
