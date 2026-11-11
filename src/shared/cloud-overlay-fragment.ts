/**
 * Carries a cloud overlay document in the URL fragment of its OBS Browser Source URL.
 * The fragment is base64url of the JSON document. The same bound applies on both
 * sides: a document that does not fit is refused, not truncated.
 */
export const MAX_FRAGMENT_CHARS = 6000;
const BASE64URL = /^[A-Za-z0-9_-]+$/;

function toBase64Url(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(value.length / 4) * 4, '=');
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}

/** Returns the base64url payload, or `null` when the document cannot fit a URL. */
export function encodeOverlayFragment(document: unknown): string | null {
  const payload = toBase64Url(JSON.stringify(document));
  return payload.length <= MAX_FRAGMENT_CHARS ? payload : null;
}

/** Reads the `overlay` parameter of a `#token=…&overlay=…` fragment. Anything malformed yields `null`. */
export function decodeOverlayFragment(hash: string): unknown {
  const params = new URLSearchParams(hash.replace(/^#/, ''));
  const payload = params.get('overlay');
  if (!payload || payload.length > MAX_FRAGMENT_CHARS || !BASE64URL.test(payload)) return null;
  try {
    return JSON.parse(fromBase64Url(payload)) as unknown;
  } catch {
    return null;
  }
}
