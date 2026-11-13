import { afterEach, describe, expect, it, vi } from 'vitest';
import { activateLocale, i18n, supportedLocales, type Locale } from './i18n.ts';
import english from './locales/en.ts';

type Tree = { [key: string]: string | Tree };

const catalogs: Record<Locale, () => Promise<{ default: Tree }>> = {
  en: () => import('./locales/en.ts') as Promise<{ default: Tree }>,
  es: () => import('./locales/es.ts') as Promise<{ default: Tree }>,
  pt: () => import('./locales/pt.ts') as Promise<{ default: Tree }>,
  fr: () => import('./locales/fr.ts') as Promise<{ default: Tree }>,
  zh: () => import('./locales/zh.ts') as Promise<{ default: Tree }>,
  ja: () => import('./locales/ja.ts') as Promise<{ default: Tree }>,
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

const placeholders = (message: string): string[] => [...message.matchAll(/\{(\w+)\}/g)].map((match) => match[1]!).sort();

afterEach(async () => {
  await activateLocale('en');
});

describe('message catalogs', () => {
  it.each(supportedLocales)('%s ships the English keys, placeholder occurrences and plural forms', async (locale) => {
    const reference = flatten(english as Tree);
    const catalog = flatten((await catalogs[locale]()).default);
    expect([...catalog.keys()].sort(), `${locale} catalog keys`).toEqual([...reference.keys()].sort());
    for (const [key, source] of reference) {
      const translated = catalog.get(key)!;
      expect(placeholders(translated), `${locale}: ${key}`).toEqual(placeholders(source));
      expect(translated.split('|').length, `${locale} plural forms: ${key}`).toBe(source.split('|').length);
    }
  });

  it.each(supportedLocales)('%s compiles every message', async (locale) => {
    const keys = [...flatten(english as Tree).keys()];
    const errors: string[] = [];
    let current = '';
    // Vue-i18n can report a compiler error and return the raw message instead of
    // throwing. Catch that failure as well, especially unescaped @ and braces.
    const logger = vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => errors.push(`${current}: ${args.map(String).join(' ')}`));
    try {
      await activateLocale(locale);
      for (const key of keys) {
        current = `${locale}:${key}`;
        expect(() => i18n.global.t(key, { count: 1, ms: 1, attempt: 1, max: 5 })).not.toThrow();
      }
      expect(errors).toEqual([]);
    } finally { logger.mockRestore(); }
  });
});
