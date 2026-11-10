import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale, i18n, isLocale, mergePluginMessages, nestMessages, supportedLocales } from './i18n.ts';

afterEach(async () => {
  await activateLocale('en');
});

describe('activateLocale', () => {
  it('lazy-loads a locale and switches to it', async () => {
    await activateLocale('es');
    expect(i18n.global.locale.value).toBe('es');
    expect(i18n.global.t('common.cancel')).toBe('Cancelar');
    expect(document.documentElement.lang).toBe('es');
  });

  it('falls back to english for the base locale', async () => {
    await activateLocale('en');
    expect(i18n.global.t('common.cancel')).toBe('Cancel');
  });
});

describe('isLocale', () => {
  it('accepts supported locales only', () => {
    expect(supportedLocales.every(isLocale)).toBe(true);
    expect(isLocale('de')).toBe(false);
    expect(isLocale(undefined)).toBe(false);
  });
});

describe('nestMessages', () => {
  it('turns dotted keys into a message tree', () => {
    expect(nestMessages({ 'plugin.title': 'Title', 'plugin.actions.run': 'Run', root: 'Root' })).toEqual({
      plugin: { title: 'Title', actions: { run: 'Run' } },
      root: 'Root',
    });
  });

  it('keeps the first value when a key collides with a branch', () => {
    expect(nestMessages({ plugin: 'Plugin', 'plugin.title': 'Title' })).toEqual({ plugin: 'Plugin' });
    expect(nestMessages({ 'plugin.title': 'Title', plugin: 'Plugin' })).toEqual({ plugin: { title: 'Title' } });
  });
});

describe('mergePluginMessages', () => {
  it('merges plugin catalogs into supported locales with interpolation', () => {
    mergePluginMessages({ en: { 'sample.greeting': 'Hello {name}' }, xx: { 'sample.greeting': 'ignored' } });
    expect(i18n.global.t('sample.greeting', { name: 'Ana' })).toBe('Hello Ana');
    expect(i18n.global.availableLocales).not.toContain('xx');
  });
});
