const INTL_LOCALES = new Map<string, string>([
  ['en', 'en-US'],
  ['es', 'es-ES'],
  ['pt', 'pt-BR'],
  ['fr', 'fr-FR'],
]);

export function intlLocale(locale: string): string {
  return INTL_LOCALES.get(locale) ?? 'en-US';
}

export function formatCount(value: number, locale: string): string {
  return Math.round(value).toLocaleString(intlLocale(locale));
}

export function formatDate(timestamp: number, locale: string): string {
  if (!timestamp) return '';
  return new Date(timestamp).toLocaleDateString(intlLocale(locale), {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}
