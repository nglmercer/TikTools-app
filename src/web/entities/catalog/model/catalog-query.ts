/**
 * Request model for the unified catalog (plan section 4.1).
 *
 * Every search carries a generation number. A reply is applied only when it
 * belongs to the newest request, so an older response can never replace the
 * results of a newer query. Cursors are tied to the query they were issued for;
 * a cursor reused with another query is dropped before it reaches the server.
 */
export type CatalogKind = 'plugin' | 'template' | 'profile';

export interface CatalogQuery {
  readonly text: string;
  readonly kinds: readonly CatalogKind[];
  readonly target?: string;
}

export interface CatalogPage<T> {
  readonly items: readonly T[];
  readonly nextCursor: string | null;
}

export function normalizeQuery(input: Partial<CatalogQuery>): CatalogQuery {
  const text = (input.text ?? '').trim().slice(0, 100);
  const kinds = [...new Set(input.kinds ?? [])].filter((kind): kind is CatalogKind =>
    kind === 'plugin' || kind === 'template' || kind === 'profile');
  const target = input.target?.trim();
  return {
    text,
    kinds: kinds.length > 0 ? kinds : ['plugin', 'template', 'profile'],
    ...(target ? { target } : {}),
  };
}

/** Stable key for a query; two queries with the same key share cursors. */
export function queryKey(query: CatalogQuery): string {
  return JSON.stringify([query.text, [...query.kinds].sort(), query.target ?? '']);
}

export interface CatalogSearch<T> {
  /** Starts a request and returns its generation. */
  begin(query: CatalogQuery): number;
  /** Applies a reply only if it is still the newest request. */
  settle(generation: number, page: CatalogPage<T>): boolean;
  /** Cursor for the current query, or null when the query is not paged. */
  cursor(): string | null;
  /** Current accepted items, empty before the first accepted reply. */
  items(): readonly T[];
  query(): CatalogQuery | null;
}

export function createCatalogSearch<T>(): CatalogSearch<T> {
  let newest = 0;
  let activeKey: string | null = null;
  let activeQuery: CatalogQuery | null = null;
  let accepted: readonly T[] = [];
  let next: string | null = null;

  return {
    begin(query) {
      newest += 1;
      activeQuery = query;
      const key = queryKey(query);
      if (key !== activeKey) {
        accepted = [];
        next = null;
        activeKey = key;
      }
      return newest;
    },
    settle(generation, page) {
      if (generation !== newest) return false;
      accepted = page.items;
      next = page.nextCursor;
      return true;
    },
    cursor() {
      return next;
    },
    items() {
      return accepted;
    },
    query() {
      return activeQuery;
    },
  };
}
