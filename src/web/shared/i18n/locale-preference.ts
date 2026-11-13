import { normalizeLocale, type Locale } from './i18n.ts';
import { appStorage } from '../lib/app-storage.ts';

const STORAGE_KEY = 'tiktok-live-locale';

export function storedLocale(): Locale | null {
  try {
    const value = appStorage().getItem(STORAGE_KEY);
    const locale = normalizeLocale(value);
    if (locale && locale !== value) {
      try { appStorage().setItem(STORAGE_KEY, locale); } catch { /* Reading remains useful in read-only storage. */ }
    }
    return locale ?? null;
  } catch {
    return null;
  }
}

export function storeLocale(locale: Locale): void {
  try {
    appStorage().setItem(STORAGE_KEY, locale);
  } catch {}
}

export function detectLocale(languages: readonly string[] = globalThis.navigator?.languages ?? []): Locale {
  const stored = storedLocale();
  if (stored) return stored;
  for (const language of languages) {
    const match = normalizeLocale(language);
    if (match) return match;
  }
  return 'en';
}
