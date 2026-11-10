import { describe, expect, it } from 'vitest';
import { decodeOverlayFragment, encodeOverlayFragment, MAX_FRAGMENT_CHARS } from '../../shared/cloud-overlay-fragment.ts';

describe('cloud overlay fragment', () => {
  it('round-trips a document through the URL fragment, including non-ASCII text', () => {
    const doc = { overlayVersion: 1, title: 'Gràcia ✓', elements: [{ id: 'a', text: 'olá' }] };
    const payload = encodeOverlayFragment(doc);
    expect(payload).not.toBeNull();
    expect(decodeOverlayFragment(`#token=abc&overlay=${payload}`)).toEqual(doc);
  });

  it('refuses a document that does not fit instead of truncating it', () => {
    expect(encodeOverlayFragment({ text: 'x'.repeat(MAX_FRAGMENT_CHARS) })).toBeNull();
  });

  it('treats a malformed or oversized fragment as absent', () => {
    expect(decodeOverlayFragment('#token=abc')).toBeNull();
    expect(decodeOverlayFragment('#overlay=not*base64')).toBeNull();
    expect(decodeOverlayFragment('#overlay=bm90IGpzb24')).toBeNull();
    expect(decodeOverlayFragment(`#overlay=${'a'.repeat(MAX_FRAGMENT_CHARS + 1)}`)).toBeNull();
  });
});
