import { describe, expect, test } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

import {
  EMOTE_CATALOG,
  EMOTE_CATALOG_VERSION,
  isKnownEmoteToken,
  type LiveEmote,
} from './index.ts';
import {
  MAX_EMOTES_PER_EVENT,
  MAX_EMOTE_URL_LENGTH,
  hasRenderableEmote,
  isSafeEmoteImageUrl,
  normalizeEmotes,
  resolveChatSegments,
  segmentsToPlainText,
} from './resolve.ts';

const SAFE_URL = 'https://cdn.example.com/emote/laughcry.webp';

function emote(overrides: Partial<LiveEmote> & { id: string }): LiveEmote {
  return { source: 'protobuf', ...overrides };
}

// The SonicBoom plugin lives in the external plugins-example-tiktools
// repository and ships its own copy of the token list. When that checkout is
// present (TIKTOOLS_PLUGINS_REPO or a sibling clone) the two must not drift.
const pluginsRepository = process.env.TIKTOOLS_PLUGINS_REPO
  ? resolve(process.env.TIKTOOLS_PLUGINS_REPO)
  : resolve(import.meta.dir, '..', '..', '..', '..', 'plugins-example-tiktools');
const pluginCatalog = join(pluginsRepository, 'plugins', 'sonicboom', 'shared', 'tts', 'emote-catalog.ts');

describe.skipIf(!existsSync(pluginCatalog))('catalog parity with the SonicBoom sanitizer', () => {
  const pluginSource = existsSync(pluginCatalog) ? readFileSync(pluginCatalog, 'utf8') : '';

  const pluginIds = [...pluginSource.matchAll(/id:\s*'([^']+)'/g)]
    .map((match) => match[1] as string)
    .sort();

  test('both catalogs list the same emote ids', () => {
    expect(pluginIds).toEqual(EMOTE_CATALOG.map((entry) => entry.id).sort());
  });

  test('both catalogs share a version', () => {
    const match = pluginSource.match(/EMOTE_CATALOG_VERSION\s*=\s*'([^']+)'/);
    expect(match?.[1]).toBe(EMOTE_CATALOG_VERSION);
  });
});

describe('isSafeEmoteImageUrl', () => {
  test('accepts absolute https URLs only', () => {
    expect(isSafeEmoteImageUrl(SAFE_URL)).toBe(true);
    expect(isSafeEmoteImageUrl('https://cdn.example.com/a.png?v=1')).toBe(true);
  });

  test('rejects privileged and plaintext schemes', () => {
    for (const url of [
      'javascript:alert(1)',
      'data:image/png;base64,AAAA',
      'file:///etc/passwd',
      'http://cdn.example.com/a.png',
      'blob:https://x/y',
      '//cdn.example.com/a.png',
      'not a url',
      '',
    ]) {
      expect(isSafeEmoteImageUrl(url)).toBe(false);
    }
  });

  test('rejects non-strings and oversized URLs', () => {
    expect(isSafeEmoteImageUrl(undefined)).toBe(false);
    expect(isSafeEmoteImageUrl(42)).toBe(false);
    expect(isSafeEmoteImageUrl(`https://cdn.example.com/${'a'.repeat(MAX_EMOTE_URL_LENGTH)}`)).toBe(
      false,
    );
  });
});

describe('resolveChatSegments', () => {
  test('plain text yields one text segment and round-trips', () => {
    const segments = resolveChatSegments('hello world');
    expect(segments).toEqual([{ type: 'text', value: 'hello world' }]);
  });

  test('empty text yields no segments', () => {
    expect(resolveChatSegments('')).toEqual([]);
  });

  test('a catalog token becomes an emote segment without inventing an image', () => {
    const segments = resolveChatSegments('Hello [laughcry] world');
    expect(segments).toEqual([
      { type: 'text', value: 'Hello ' },
      {
        type: 'emote',
        id: 'laughcry',
        shortcode: '[laughcry]',
        imageUrl: undefined,
        fallbackText: '[laughcry]',
      },
      { type: 'text', value: ' world' },
    ]);
  });

  test('event metadata supplies the image URL and wins over the catalog', () => {
    const segments = resolveChatSegments('[laughcry]', [
      emote({ id: 'laughcry', shortcode: '[laughcry]', imageUrl: SAFE_URL }),
    ]);
    expect(segments).toEqual([
      {
        type: 'emote',
        id: 'laughcry',
        shortcode: '[laughcry]',
        imageUrl: SAFE_URL,
        fallbackText: '[laughcry]',
      },
    ]);
    expect(hasRenderableEmote(segments)).toBe(true);
  });

  test('a custom emote with its own token is rendered', () => {
    const segments = resolveChatSegments('go [creatorfire]!', [
      emote({ id: '999001', shortcode: '[creatorfire]', imageUrl: SAFE_URL, packageId: 'room-7' }),
    ]);
    expect(segments[1]).toMatchObject({ type: 'emote', id: '999001', imageUrl: SAFE_URL });
  });

  test('an unsafe image URL keeps the emote but drops the image', () => {
    const segments = resolveChatSegments('[wow]', [
      emote({ id: 'wow', shortcode: '[wow]', imageUrl: 'javascript:alert(1)' }),
    ]);
    expect(segments).toEqual([
      {
        type: 'emote',
        id: 'wow',
        shortcode: '[wow]',
        imageUrl: undefined,
        fallbackText: '[wow]',
      },
    ]);
    expect(hasRenderableEmote(segments)).toBe(false);
  });

  test('unknown bracketed text stays literal', () => {
    for (const comment of ['Array [1,2,3]', 'Test [debug]', '[username] joined']) {
      expect(resolveChatSegments(comment)).toEqual([{ type: 'text', value: comment }]);
    }
  });

  test('adjacent emotes become separate segments', () => {
    const segments = resolveChatSegments('[wow][wow]');
    expect(segments).toHaveLength(2);
    expect(segments.every((segment) => segment.type === 'emote')).toBe(true);
  });

  test('emotes mixed with text and Unicode keep order', () => {
    const segments = resolveChatSegments('Hi 😀 [happy] ¿qué tal? [wow]');
    expect(segments.map((segment) => segment.type)).toEqual([
      'text',
      'emote',
      'text',
      'emote',
    ]);
    expect(segmentsToPlainText(segments)).toBe('Hi 😀 [happy] ¿qué tal? [wow]');
  });

  test('unmatched metadata follows unchanged text', () => {
    const segments = resolveChatSegments('no token here', [
      emote({ id: 'laughcry', shortcode: '[laughcry]', imageUrl: SAFE_URL }),
    ]);
    expect(segments[0]).toEqual({ type: 'text', value: 'no token here' });
    expect(segments[1]).toMatchObject({ type: 'emote', imageUrl: SAFE_URL, fallbackText: '' });
    expect(segmentsToPlainText(segments)).toBe('no token here');
  });

  test('an unclosed bracket is preserved as text', () => {
    expect(resolveChatSegments('Hello [laughcry')).toEqual([
      { type: 'text', value: 'Hello [laughcry' },
    ]);
  });

  test('oversized tokens are never matched', () => {
    const token = `[${'x'.repeat(64)}]`;
    expect(resolveChatSegments(`a ${token} b`)).toEqual([{ type: 'text', value: `a ${token} b` }]);
  });

  test('the emote count is bounded', () => {
    const many: LiveEmote[] = Array.from({ length: MAX_EMOTES_PER_EVENT + 20 }, (_, index) =>
      emote({ id: `custom-${index}`, shortcode: `[custom-${index}]`, imageUrl: SAFE_URL }),
    );
    const tokens = many.map((entry) => entry.shortcode).join('');
    const segments = resolveChatSegments(tokens, many);
    const emoteCount = segments.filter((segment) => segment.type === 'emote').length;
    expect(emoteCount).toBe(MAX_EMOTES_PER_EVENT);
    // Everything past the bound falls back to text, never to a fake emote.
    expect(segmentsToPlainText(segments)).toBe(tokens);
  });

  test('invalid metadata entries are dropped', () => {
    const segments = resolveChatSegments('[ok] [bad]', [
      emote({ id: '' }),
      emote({ id: 'ok', shortcode: '[ok]', imageUrl: SAFE_URL }),
    ]);
    expect(segments[0]).toMatchObject({ type: 'emote', id: 'ok' });
    // `[bad]` is not a known token, so it stays text.
    expect(segments[1]).toEqual({ type: 'text', value: ' [bad]' });
  });
});

describe('segmentsToPlainText invariant', () => {
  test('always reproduces the original comment', () => {
    for (const comment of [
      'hello',
      'Hello [laughcry] world',
      '[wow][happy]',
      'Array [1,2,3] and [debug]',
      'こんにちは [happy] 世界',
      'a [b [c] d',
      '',
    ]) {
      expect(segmentsToPlainText(resolveChatSegments(comment))).toBe(comment);
    }
  });
});

describe('normalizeEmotes', () => {
  test('keys by the lowercased token', () => {
    const map = normalizeEmotes([emote({ id: 'laughcry', shortcode: '[LaughCry]' })]);
    expect(map.get('laughcry')?.id).toBe('laughcry');
  });

  test('defaults the token to the id when no shortcode is given', () => {
    const map = normalizeEmotes([emote({ id: 'wow' })]);
    expect(map.get('wow')?.shortcode).toBe('[wow]');
  });

  test('returns an empty map for absent input', () => {
    expect(normalizeEmotes(undefined).size).toBe(0);
    expect(normalizeEmotes([]).size).toBe(0);
  });
});

describe('catalog shape', () => {
  test('ids and shortcodes are consistent', () => {
    const seen = new Set<string>();
    for (const entry of EMOTE_CATALOG) {
      expect(seen.has(entry.id)).toBe(false);
      seen.add(entry.id);
      expect(entry.id).toBe(entry.id.toLowerCase());
      expect(entry.shortcode).toBe(`[${entry.id}]`);
    }
    expect(isKnownEmoteToken('HAPPY')).toBe(true);
    expect(isKnownEmoteToken('nope')).toBe(false);
  });
});

describe('wire emote metadata', () => {
  test('unmatched opaque ids keep images and preserve multilingual Unicode text', () => {
    const text = 'Hola 👩🏽‍💻 世界';
    const segments = resolveChatSegments(text, [
      { id: '9007199254740993', imageUrl: SAFE_URL, index: 999, packageId: 'creator' },
      { id: '9007199254740993', imageUrl: SAFE_URL },
    ]);
    expect(segmentsToPlainText(segments)).toBe(text);
    expect(segments.filter((segment) => segment.type === 'emote')).toHaveLength(2);
    expect(segments[1]).toMatchObject({ id: '9007199254740993', imageUrl: SAFE_URL, fallbackText: '' });
  });
  test('an indexed token consumes exactly one metadata occurrence', () => {
    const segments = resolveChatSegments('😀 [custom]', [{ id: '123', imageUrl: SAFE_URL, index: 3 }]);
    expect(segments.filter((segment) => segment.type === 'emote')).toHaveLength(1);
    expect(segmentsToPlainText(segments)).toBe('😀 [custom]');
  });
  test('emote-only messages retain opaque emotes even without artwork', () => {
    expect(resolveChatSegments('', [{ id: '123' }])).toMatchObject([{ type: 'emote', id: '123' }]);
  });
});
