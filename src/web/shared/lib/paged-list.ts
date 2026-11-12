import { computed, ref, shallowRef } from 'vue';
import { createRequestScope } from './request-scope.ts';

export interface PagedReply<T> {
  readonly items: readonly T[];
  readonly nextCursor: string | null;
}

export interface PagedListOptions<T> {
  /** Reads one page; `after` is the previous reply's `nextCursor`, or null for the first page. */
  fetch(after: string | null): Promise<PagedReply<T>>;
  /** Stable identity, used to drop an item that two pages both returned. */
  key(item: T): string;
}

/**
 * A cursor-paged list that only ever shows the newest answer. A refresh supersedes any page
 * still in flight, and a "load more" is dropped when a refresh, a reset or another "load more"
 * happened meanwhile, so a slow reply can never put old rows back. Failures are kept as the
 * thrown value (`error`), so each owner can map them to its own wording.
 */
export function createPagedList<T>(options: PagedListOptions<T>) {
  const scope = createRequestScope();
  const items = shallowRef<readonly T[]>([]);
  const cursor = ref<string | null>(null);
  const loading = ref(false);
  const loaded = ref(false);
  const error = shallowRef<unknown>(null);

  async function load(append: boolean): Promise<void> {
    const after = append ? cursor.value : null;
    if (append && after === null) return;
    const ticket = scope.begin();
    loading.value = true;
    error.value = null;
    try {
      const page = await options.fetch(after);
      if (!ticket.isCurrent()) return;
      if (append) {
        const known = new Set(items.value.map(options.key));
        items.value = [...items.value, ...page.items.filter((item) => !known.has(options.key(item)))];
      } else {
        items.value = [...page.items];
      }
      cursor.value = page.nextCursor;
      loaded.value = true;
    } catch (failure) {
      if (ticket.isCurrent()) error.value = failure;
    } finally {
      if (ticket.isCurrent()) loading.value = false;
    }
  }

  /** Replaces one row after a change the Cloud confirmed. */
  function replace(next: T): void {
    const id = options.key(next);
    items.value = items.value.map((item) => (options.key(item) === id ? next : item));
  }

  /** Drops one row after a delete the Cloud confirmed. */
  function remove(id: string): void {
    items.value = items.value.filter((item) => options.key(item) !== id);
  }

  /** Forgets everything and makes every pending reply stale (a new account, session or host). */
  function reset(): void {
    scope.invalidate();
    items.value = [];
    cursor.value = null;
    loading.value = false;
    loaded.value = false;
    error.value = null;
  }

  return {
    items,
    loading,
    loaded,
    error,
    hasMore: computed(() => cursor.value !== null),
    refresh: () => load(false),
    loadMore: () => load(true),
    replace,
    remove,
    reset,
  };
}

export type PagedList<T> = ReturnType<typeof createPagedList<T>>;
