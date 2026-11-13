import { createI18n } from 'vue-i18n';
import type { TranslationCatalog } from '../../../shared/localized.ts';
import en from './locales/en.ts';
import { isLocale, type Locale } from './supported-locales.ts';
export { supportedLocales, isLocale, normalizeLocale, type Locale } from './supported-locales.ts';

export type MessageSchema = typeof en;


const fallbackLocale = 'en' satisfies Locale;

const loaders: Record<Exclude<Locale, typeof fallbackLocale>, () => Promise<{ default: MessageSchema }>> = {
  es: () => import('./locales/es.ts'),
  pt: () => import('./locales/pt.ts'),
  fr: () => import('./locales/fr.ts'),
  zh: () => import('./locales/zh.ts'),
  ja: () => import('./locales/ja.ts'),
};

// Only display strings for host-owned keys. Native descriptors, capabilities,
// field paths and executable behavior continue to come from the host.
const hostLoaders: Partial<Record<Locale, () => Promise<{ default: Record<string, string> }>>> = {
  zh: () => import('./host-messages/zh.ts'),
  ja: () => import('./host-messages/ja.ts'),
};

declare module 'vue-i18n' {
  export interface DefineLocaleMessage extends MessageSchema {}
}

export const i18n = createI18n<[MessageSchema], Locale, false>({
  legacy: false,
  locale: fallbackLocale,
  fallbackLocale,
  messages: { en } as Record<Locale, MessageSchema>,
});

const loaded = new Set<Locale>();


export async function activateLocale(locale: Locale): Promise<void> {
  if (locale !== fallbackLocale && !loaded.has(locale)) {
    const [messages, host] = await Promise.all([loaders[locale](), hostLoaders[locale]?.()]);
    i18n.global.setLocaleMessage(locale, messages.default);
    if (host) i18n.global.mergeLocaleMessage(locale, nestMessages(host.default));
    loaded.add(locale);
  }
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}

type MessageTree = { [key: string]: string | MessageTree };

export function nestMessages(flat: Record<string, string>): MessageTree {
  const tree: MessageTree = {};
  for (const [key, value] of Object.entries(flat)) {
    const path = key.split('.');
    const leaf = path.pop()!;
    let node = tree;
    let blocked = false;
    for (const segment of path) {
      const next = node[segment];
      if (typeof next === 'string') {
        blocked = true;
        break;
      }
      node = next ?? (node[segment] = {});
    }
    if (!blocked && typeof node[leaf] !== 'object') node[leaf] = value;
  }
  return tree;
}

export function mergePluginMessages(catalog: TranslationCatalog): void {
  for (const [locale, messages] of Object.entries(catalog)) {
    if (isLocale(locale)) i18n.global.mergeLocaleMessage(locale, nestMessages(messages));
  }
}

export function localizedText(value: unknown): string {
  if (typeof value === 'string') return value;
  if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
  const { i18key, default: fallback } = value as { i18key?: unknown; default?: unknown };
  if (typeof i18key === 'string' && i18key && (i18n.global.te(i18key) || i18n.global.te(i18key, fallbackLocale))) {
    try {
      return i18n.global.t(i18key);
    } catch {
      return typeof fallback === 'string' ? fallback : i18key;
    }
  }
  return typeof fallback === 'string' ? fallback : '';
}
