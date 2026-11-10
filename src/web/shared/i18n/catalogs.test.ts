import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale, i18n, supportedLocales, type Locale } from './i18n.ts';
import english from './locales/en.ts';

type Tree = { [key: string]: string | Tree };

const catalogs: Record<Locale, () => Promise<{ default: Tree }>> = {
  en: () => import('./locales/en.ts') as Promise<{ default: Tree }>,
  es: () => import('./locales/es.ts') as Promise<{ default: Tree }>,
  pt: () => import('./locales/pt.ts') as Promise<{ default: Tree }>,
  fr: () => import('./locales/fr.ts') as Promise<{ default: Tree }>,
};

function flatten(tree: Tree, prefix = ''): Map<string, string> {
  const leaves = new Map<string, string>();
  for (const [key, value] of Object.entries(tree)) {
    const path = `${prefix}${key}`;
    if (typeof value === 'string') leaves.set(path, value);
    else for (const [leafKey, leafValue] of flatten(value, `${path}.`)) leaves.set(leafKey, leafValue);
  }
  return leaves;
}

const placeholders = (message: string): string[] => [...new Set([...message.matchAll(/\{(\w+)\}/g)].map((match) => match[1]!))].sort();

afterEach(async () => {
  await activateLocale('en');
});

describe('message catalogs', () => {
  it('ships every supported locale with the english keys, placeholders and plural forms', async () => {
    const reference = flatten(english as Tree);
    for (const locale of supportedLocales) {
      const catalog = flatten((await catalogs[locale]()).default);
      expect([...reference.keys()].filter((key) => !catalog.has(key)), `${locale} missing keys`).toEqual([]);
      for (const [key, source] of reference) {
        const translated = catalog.get(key)!;
        expect(placeholders(translated), `${locale}: ${key}`).toEqual(placeholders(source));
        expect(translated.split('|').length, `${locale} plural forms: ${key}`).toBe(source.split('|').length);
      }
    }
  });

  it('compiles every message in every locale', async () => {
    const keys = [...flatten(english as Tree).keys()];
    for (const locale of supportedLocales) {
      await activateLocale(locale);
      for (const key of keys) {
        expect(() => i18n.global.t(key, { count: 1, ms: 1, attempt: 1, max: 5 })).not.toThrow();
      }
    }
  });
});
