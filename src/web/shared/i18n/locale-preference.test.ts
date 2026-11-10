import { afterEach, describe, expect, it } from 'vitest';
import { detectLocale, storedLocale, storeLocale } from './locale-preference.ts';

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
    localStorage.setItem('tiktok-live-locale', 'de');
    expect(storedLocale()).toBeNull();
  });

  it('matches the first supported system language', () => {
    expect(detectLocale(['de-DE', 'fr-FR', 'es-MX'])).toBe('fr');
    expect(detectLocale(['pt-BR'])).toBe('pt');
    expect(detectLocale(['de-DE'])).toBe('en');
  });
});
