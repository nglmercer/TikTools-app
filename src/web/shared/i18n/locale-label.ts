import { i18n } from './i18n.ts';

/**
 * The name a language is listed under: written in that language itself, so a reader can always find
 * theirs. A catalog entry (`menu.languages.<code>`) wins; a locale without one falls back to the
 * platform's own name for it and finally to the bare code, so adding a locale never shows a key.
 */
export function localeLabel(code: string): string {
  const key = `menu.languages.${code}`;
  if (i18n.global.te(key)) return i18n.global.t(key);
  try {
    const name = new Intl.DisplayNames([code], { type: 'language' }).of(code);
    if (name && name !== code) return name.charAt(0).toLocaleUpperCase(code) + name.slice(1);
  } catch {
    // Not a language tag the platform knows.
  }
  return code;
}
