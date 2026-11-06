/**
 * Known TikTok LIVE emote shortcodes, app side.
 *
 * This is the same token set the SonicBoom sanitizer uses
 * (`plugins/sonicboom/shared/tts/emote-catalog.ts` in plugins-example-tiktools). The two live in separate
 * build graphs — the plugin UI ships its own bundle and never imports the main
 * app — so the list is duplicated, and `catalog.test.ts` reads the plugin file
 * and fails if the two ever disagree.
 *
 * No `imageUrl` is set on purpose. The signer can supply artwork per event;
 * catalog-only entries stay plain text until an ID-to-artwork mapping is
 * verified against captured LIVE traffic.
 */

export type CatalogEmote = {
  /** Lowercase lookup id, e.g. `laughcry`. */
  id: string;
  /** Token as it appears in chat, e.g. `[laughcry]`. */
  shortcode: string;
  /**
   * Verified HTTPS artwork. Intentionally absent until the mapping can be
   * confirmed; do not populate speculatively.
   */
  imageUrl?: string;
};

/** Bump whenever the token set changes; must match the plugin catalog. */
export const EMOTE_CATALOG_VERSION = '2026.10.0';

/** Longest shortcode the scanner will consider. */
export const MAX_EMOTE_TOKEN_LENGTH = 32;

export const EMOTE_CATALOG: readonly CatalogEmote[] = Object.freeze([
  { id: 'laughcry', shortcode: '[laughcry]' },
  { id: 'happy', shortcode: '[happy]' },
  { id: 'wow', shortcode: '[wow]' },
  { id: 'heart', shortcode: '[heart]' },
  { id: 'cry', shortcode: '[cry]' },
  { id: 'laugh', shortcode: '[laugh]' },
  { id: 'angry', shortcode: '[angry]' },
  { id: 'shock', shortcode: '[shock]' },
  { id: 'thumbsup', shortcode: '[thumbsup]' },
  { id: 'fire', shortcode: '[fire]' },
]);

const byId: ReadonlyMap<string, CatalogEmote> = new Map(
  EMOTE_CATALOG.map((entry) => [entry.id, entry]),
);

/** Case-folded lookup, matching the SonicBoom sanitizer's matching rule. */
export function catalogEmoteById(id: string): CatalogEmote | undefined {
  const key = id.trim().toLowerCase();
  return key ? byId.get(key) : undefined;
}

export function isKnownEmoteToken(token: string): boolean {
  return byId.has(token.trim().toLowerCase());
}
