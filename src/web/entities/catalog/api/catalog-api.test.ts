import { describe, expect, it } from 'vitest';
import { catalogItemsParams, InvalidCatalogResponse, parseCatalogItem, parseCatalogPage } from './catalog-api.ts';
import { normalizeQuery } from '../model/catalog-query.ts';

const plugin = {
  identity: { kind: 'plugin', id: 'acme.gift' },
  presentation: { title: 'Gift alerts', summary: 'Shows gifts', publisher: 'Acme', categories: ['alerts'], authors: ['x'] },
  compatibility: { targets: ['windows', 'linux'] },
  releaseRef: { type: 'pluginRelease', generation: '9', version: '1.2.0', target: 'any', sha256: 'a', size: '42' },
  publication: { status: 'published', yanked: false },
};

describe('catalog item parsing', () => {
  it('maps the wire item to the summary and keeps presentation separate from install state', () => {
    const item = parseCatalogItem(plugin);
    expect(item).toMatchObject({ kind: 'plugin', id: 'acme.gift', title: 'Gift alerts', version: '1.2.0', yanked: false });
    expect(item.targets).toEqual(['windows', 'linux']);
  });

  it('falls back to the identifier when a release has no display title', () => {
    const item = parseCatalogItem({ ...plugin, presentation: { summary: '' } });
    expect(item.title).toBe('acme.gift');
  });

  it('reads a version only from a plugin release', () => {
    const template = parseCatalogItem({
      identity: { kind: 'template', id: 'tpl-1' },
      releaseRef: { type: 'documentRevision', revision: 3, sha256: 'b' },
    });
    expect(template.version).toBeNull();
  });

  it('refuses items whose kind or identifier cannot be rendered', () => {
    expect(() => parseCatalogItem({ identity: { kind: 'overlay', id: 'x' } })).toThrow(InvalidCatalogResponse);
    expect(() => parseCatalogItem({ identity: { kind: 'plugin', id: '' } })).toThrow(InvalidCatalogResponse);
    expect(() => parseCatalogItem(null)).toThrow(InvalidCatalogResponse);
  });

  it('validates the page cursor type', () => {
    expect(parseCatalogPage({ items: [], nextCursor: null }).nextCursor).toBeNull();
    expect(parseCatalogPage({ items: [] }).nextCursor).toBeNull();
    expect(() => parseCatalogPage({ items: [], nextCursor: 7 })).toThrow(InvalidCatalogResponse);
    expect(() => parseCatalogPage({ nextCursor: null })).toThrow(InvalidCatalogResponse);
  });
});

describe('catalog request params', () => {
  it('sends one kind and the host target when a single type is selected', () => {
    const params = catalogItemsParams(normalizeQuery({ text: 'gift', kinds: ['template'], target: 'windows' }), null);
    expect(params).toEqual({ query: 'gift', limit: 25, kind: 'template', target: 'windows' });
  });

  it('omits the kind when every type is selected', () => {
    const params = catalogItemsParams(normalizeQuery({ text: '' }), 'c1');
    expect(params).toEqual({ query: '', limit: 25, cursor: 'c1' });
  });

  it('refuses a two-kind query instead of widening it', () => {
    expect(() => catalogItemsParams(normalizeQuery({ kinds: ['plugin', 'profile'] }), null)).toThrow(/one kind/);
  });
});
