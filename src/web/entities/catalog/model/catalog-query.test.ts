import { describe, expect, it } from 'vitest';
import { createCatalogSearch, normalizeQuery, queryKey } from './catalog-query.ts';

describe('catalog query', () => {
  it('bounds the text, deduplicates kinds and defaults to every kind', () => {
    const query = normalizeQuery({ text: `  ${'x'.repeat(150)}  `, kinds: ['plugin', 'plugin', 'template'] as never });
    expect(query.text.length).toBe(100);
    expect(query.kinds).toEqual(['plugin', 'template']);
    expect(normalizeQuery({ kinds: ['overlay'] as never }).kinds).toEqual(['plugin', 'template', 'profile']);
  });

  it('keys identical queries the same regardless of kind order', () => {
    const a = normalizeQuery({ text: 'gift', kinds: ['template', 'plugin'] });
    const b = normalizeQuery({ text: 'gift', kinds: ['plugin', 'template'] });
    expect(queryKey(a)).toBe(queryKey(b));
    expect(queryKey(a)).not.toBe(queryKey(normalizeQuery({ text: 'gift', kinds: ['plugin'] })));
  });
});

describe('catalog search', () => {
  it('applies only the reply to the newest request', () => {
    const search = createCatalogSearch<string>();
    const old = search.begin(normalizeQuery({ text: 'a' }));
    const newer = search.begin(normalizeQuery({ text: 'ab' }));
    expect(search.settle(newer, { items: ['ab-result'], nextCursor: null })).toBe(true);
    expect(search.settle(old, { items: ['a-result'], nextCursor: 'stale' })).toBe(false);
    expect(search.items()).toEqual(['ab-result']);
    expect(search.cursor()).toBeNull();
  });

  it('clears accepted items when the query changes, before the new reply arrives', () => {
    const search = createCatalogSearch<string>();
    const first = search.begin(normalizeQuery({ text: 'one' }));
    search.settle(first, { items: ['one-result'], nextCursor: 'c1' });
    search.begin(normalizeQuery({ text: 'two' }));
    expect(search.items()).toEqual([]);
    expect(search.cursor()).toBeNull();
  });

  it('keeps items for a repeated query while it is refreshed', () => {
    const search = createCatalogSearch<string>();
    const first = search.begin(normalizeQuery({ text: 'same' }));
    search.settle(first, { items: ['kept'], nextCursor: 'c1' });
    search.begin(normalizeQuery({ text: 'same' }));
    expect(search.items()).toEqual(['kept']);
    expect(search.cursor()).toBe('c1');
  });
});
