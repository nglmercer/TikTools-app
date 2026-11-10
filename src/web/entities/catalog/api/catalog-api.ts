import type { ControlClient } from '@/shared/api/control-client.ts';
import type { CatalogKind, CatalogPage, CatalogQuery } from '../model/catalog-query.ts';

/**
 * Presentation of one unified catalog item. It describes what the item is,
 * never whether it may be installed: installation always goes through the
 * signed package review.
 */
export interface CatalogItemSummary {
  readonly kind: CatalogKind;
  readonly id: string;
  readonly title: string;
  readonly summary: string;
  readonly publisher: string | null;
  readonly categories: readonly string[];
  readonly targets: readonly string[];
  /** Version of the published plugin release, when the item is a plugin. */
  readonly version: string | null;
  readonly yanked: boolean;
}

export const CATALOG_PAGE_SIZE = 25;
const KINDS: readonly CatalogKind[] = ['plugin', 'template', 'profile'];
const MAX_LIST = 64;

type Json = Record<string, unknown>;

export class InvalidCatalogResponse extends Error {
  constructor() {
    super('invalid catalog response');
    this.name = 'InvalidCatalogResponse';
  }
}

function record(value: unknown): Json | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Json) : null;
}

function text(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback;
}

function strings(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((entry): entry is string => typeof entry === 'string').slice(0, MAX_LIST);
}

/** Maps one wire item to the summary the UI renders. Unknown fields are dropped. */
export function parseCatalogItem(value: unknown): CatalogItemSummary {
  const item = record(value);
  const identity = record(item?.identity);
  const kind = identity?.kind;
  const id = identity?.id;
  if (typeof kind !== 'string' || !KINDS.includes(kind as CatalogKind)) throw new InvalidCatalogResponse();
  if (typeof id !== 'string' || id.length === 0) throw new InvalidCatalogResponse();

  const presentation = record(item?.presentation);
  const compatibility = record(item?.compatibility);
  const release = record(item?.releaseRef);
  const publication = record(item?.publication);
  const pluginRelease = release?.type === 'pluginRelease';
  return {
    kind: kind as CatalogKind,
    id,
    title: text(presentation?.title, id),
    summary: text(presentation?.summary),
    publisher: typeof presentation?.publisher === 'string' ? presentation.publisher : null,
    categories: strings(presentation?.categories),
    targets: strings(compatibility?.targets),
    version: pluginRelease && typeof release?.version === 'string' ? release.version : null,
    yanked: publication?.yanked === true,
  };
}

export function parseCatalogPage(value: unknown): CatalogPage<CatalogItemSummary> {
  const page = record(value);
  if (!page || !Array.isArray(page.items)) throw new InvalidCatalogResponse();
  const cursor = page.nextCursor;
  if (cursor !== null && cursor !== undefined && typeof cursor !== 'string') throw new InvalidCatalogResponse();
  return {
    items: page.items.map(parseCatalogItem),
    nextCursor: typeof cursor === 'string' ? cursor : null,
  };
}

/**
 * The v3 endpoint takes one kind per request. The toolbar only produces one
 * kind or all of them; any other combination is refused rather than silently
 * widened.
 */
export function catalogItemsParams(query: CatalogQuery, cursor: string | null): Record<string, unknown> {
  if (query.kinds.length !== 1 && query.kinds.length !== KINDS.length) {
    throw new Error('the catalog accepts one kind per request');
  }
  return {
    query: query.text,
    limit: CATALOG_PAGE_SIZE,
    ...(query.kinds.length === 1 ? { kind: query.kinds[0] } : {}),
    ...(query.target ? { target: query.target } : {}),
    ...(cursor ? { cursor } : {}),
  };
}

export async function fetchCatalogPage(
  client: ControlClient,
  query: CatalogQuery,
  cursor: string | null,
): Promise<CatalogPage<CatalogItemSummary>> {
  return parseCatalogPage(await client.call<unknown>('catalog.items', catalogItemsParams(query, cursor)));
}

export async function fetchCatalogItem(
  client: ControlClient,
  kind: CatalogKind,
  id: string,
  target?: string,
): Promise<CatalogItemSummary> {
  return parseCatalogItem(
    await client.call<unknown>('catalog.item', { kind, id, ...(target ? { target } : {}) }),
  );
}
