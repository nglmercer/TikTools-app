/**
 * SonicBoom plugin UI localization (standalone copy of the host `t()`
 * contract, scoped to this package's keys). The plugin UI never imports
 * the main frontend, so it ships its own tiny catalog instead.
 */

import { english, type TranslationKey } from './en.ts';
import { spanish } from './es.ts';

export const supportedLocales = ['en', 'es'] as const;
export type Locale = (typeof supportedLocales)[number];
export type { TranslationKey };

const dictionary: Record<Locale, Record<string, string>> = {
  en: { ...english },
  es: { ...spanish },
};

export function t(
  locale: Locale,
  key: TranslationKey | string,
  values: Record<string, string | number> = {},
): string {
  const item = dictionary[locale]?.[key as TranslationKey]
    ?? dictionary.en?.[key as TranslationKey]
    ?? key;
  return item.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    const value = values[name];
    return value === undefined ? placeholder : String(value);
  });
}
