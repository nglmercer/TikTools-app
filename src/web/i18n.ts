import type { TranslationCatalog } from '../automation/behavior/types.ts';
import { BEHAVIOR_UI_TRANSLATIONS } from './behavior-i18n.ts';
import { english, type TranslationKey } from './i18n-en.ts';
import { spanish } from './i18n-es.ts';

export const supportedLocales = ['en', 'es'] as const;
export type Locale = (typeof supportedLocales)[number];
export type { TranslationKey };

const dictionary: Record<Locale, Record<string, string>> = {
  en: { ...english, ...(BEHAVIOR_UI_TRANSLATIONS.en ?? {}) },
  es: { ...spanish, ...(BEHAVIOR_UI_TRANSLATIONS.es ?? {}) },
};

let pluginDictionary: TranslationCatalog = {};

/**
 * Replace the optional plugin catalog delivered by the host. Keeping this in
 * the WebView avoids coupling plugin packages to Vue or to the DOM.
 */
export function setPluginTranslations(catalog: TranslationCatalog | undefined): void {
  pluginDictionary = catalog ?? {};
}

/** Resolve host or plugin metadata using `{ default, i18key }`. */
export function i18nText(locale: Locale, value: unknown): string {
  if (typeof value === 'string') return value;
  if (!value || typeof value !== 'object' || Array.isArray(value)) return '';

  const object = value as Record<string, unknown>;
  const key = typeof object.i18key === 'string' ? object.i18key : '';
  if (key) {
    const localized = dictionary[locale]?.[key as TranslationKey]
      ?? pluginDictionary[locale]?.[key]
      ?? dictionary.en?.[key as TranslationKey]
      ?? pluginDictionary.en?.[key];
    if (localized) return localized;
  }

  if (typeof object.default === 'string') return object.default;
  return '';
}

export function t(locale: Locale, key: TranslationKey | string, values: Record<string, string | number> = {}): string {
  const item = dictionary[locale]?.[key as TranslationKey]
    ?? pluginDictionary[locale]?.[key]
    ?? dictionary.en?.[key as TranslationKey]
    ?? pluginDictionary.en?.[key]
    ?? key;
  return item.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    const value = values[name];
    return value === undefined ? placeholder : String(value);
  });
}
