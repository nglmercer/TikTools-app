import en from './locales/en.ts';
import es from './locales/es.ts';
import pt from './locales/pt.ts';
import fr from './locales/fr.ts';
import zh from './locales/zh.ts';
import ja from './locales/ja.ts';
import hostJa from './host-messages/ja.ts';
import hostZh from './host-messages/zh.ts';
import type { Locale } from './supported-locales.ts';

/**
 * Resolve a UI string from the real message catalogs in tests.
 *
 * Tests must never hardcode a translated label (neither `locale === 'en' ?
 * 'Save' : 'Guardar'` nor a single-locale string): when a translation changes,
 * the assertion still describes the behavior, not a stale copy of the text.
 * This helper is deliberately plain TypeScript so it works in Vitest and in
 * Playwright specs (no Vue instance, no `document`).
 */
const catalogs: Record<Locale, unknown> = { en, es, pt, fr, zh, ja };
/** CJK host descriptors are merged into the catalog at runtime by i18n.ts. */
const hostMessages: Partial<Record<Locale, Record<string, string>>> = { ja: hostJa, zh: hostZh };
type Tree = { [key: string]: string | Tree };

function leaf(value: unknown, key: string): string | undefined {
  let node = value;
  for (const segment of key.split('.')) {
    if (!node || typeof node !== 'object') return undefined;
    node = (node as Tree)[segment];
  }
  return typeof node === 'string' ? node : undefined;
}

/** Pick the first plural form; tests assert on presence, not grammar. */
function plural(text: string, count?: number): string {
  const forms = text.split('|');
  if (forms.length === 1 || count === undefined) return forms[0]!;
  return forms[Math.min(count, forms.length - 1)]!;
}

export function testMessage(locale: Locale, key: string, params: Record<string, string | number> = {}): string {
  const value = leaf(catalogs[locale], key) ?? hostMessages[locale]?.[key] ?? leaf(en, key);
  if (value === undefined) throw new Error(`Missing i18n key in test: ${key} (${locale})`);
  const count = typeof params.count === 'number' ? params.count : undefined;
  return plural(value, count)
    // vue-i18n literal form `{'@'}`: keep the quoted character verbatim.
    .replace(/\{'([^']*)'\}/g, (_match, literal: string) => literal)
    .replace(/\{(\w+)\}/g, (match, name: string) =>
      params[name] === undefined ? match : String(params[name]),
    );
}
