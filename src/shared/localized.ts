/**
 * Canonical localized-text types shared by every domain.
 *
 * `I18nText`/`Localized` describe `{ default, i18key }` metadata attached
 * to plugin descriptors, UI nodes, and automation records. The contract
 * is domain-neutral, so it lives in `src/shared/` instead of automation.
 */

import type { JsonObject } from './json.ts';

/**
 * Localized metadata is intentionally a small, serializable value object.
 *
 * `default` keeps a plugin usable when its optional locale file is missing;
 * `i18key` is the stable lookup key used by the host translation catalog.
 * Plugins should namespace keys with their plugin id.
 */
export interface I18nText extends JsonObject {
  default: string;
  i18key: string;
}

/** Every localized descriptor uses the default/key contract. */
export type Localized = I18nText;

/** Locale -> key -> translated value. Locale files use this exact shape. */
export type TranslationCatalog = Record<string, Record<string, string>>;
