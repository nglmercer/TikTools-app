/** Pure locale metadata: safe for documentation SSR and demo parameter parsing. */
export const supportedLocales = ['en', 'es', 'pt', 'fr', 'zh', 'ja'] as const;
export type Locale = typeof supportedLocales[number];
export const localeNames = {
  en: 'English', es: 'Español', pt: 'Português', fr: 'Français', zh: '简体中文', ja: '日本語',
} as const satisfies Record<Locale, string>;

export function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && (supportedLocales as readonly string[]).includes(value);
}

/** Accept language tags and the commonly requested `jp` alias; persist canonical IDs. */
export function normalizeLocale(value: unknown): Locale | undefined {
  if (typeof value !== 'string') return;
  const base = value.trim().toLowerCase().replaceAll('_', '-').split('-')[0];
  const code = base === 'jp' ? 'ja' : base;
  return isLocale(code) ? code : undefined;
}
