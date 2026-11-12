import { describe, expect, it } from 'vitest';
import { createPagedList, type PagedReply } from './paged-list.ts';

interface Row { id: string; label?: string }
const row = (id: string): Row => ({ id });

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, no) => { resolve = ok; reject = no; });
  return { promise, resolve, reject };
}

describe('paged list', () => {
  it('loads the first page and appends the next one without repeating a row', async () => {
    const pages: Record<string, PagedReply<Row>> = {
      first: { items: [row('a'), row('b')], nextCursor: 'c1' },
      c1: { items: [row('b'), row('c')], nextCursor: null },
    };
    const seen: (string | null)[] = [];
    const list = createPagedList<Row>({ key: (item) => item.id, fetch: async (after) => { seen.push(after); return pages[after ?? 'first']!; } });
    await list.refresh();
    expect(list.items.value.map((item) => item.id)).toEqual(['a', 'b']);
    expect(list.hasMore.value).toBe(true);
    await list.loadMore();
    expect(list.items.value.map((item) => item.id)).toEqual(['a', 'b', 'c']);
    expect(list.hasMore.value).toBe(false);
    expect(seen).toEqual([null, 'c1']);
    await list.loadMore();
    expect(seen).toEqual([null, 'c1']);
  });

  it('drops a slow older reply when a newer refresh was answered first', async () => {
    const slow = deferred<PagedReply<Row>>();
    const replies = [slow.promise, Promise.resolve({ items: [row('new')], nextCursor: null })];
    const list = createPagedList<Row>({ key: (item) => item.id, fetch: () => replies.shift()! });
    const first = list.refresh();
    await list.refresh();
    slow.resolve({ items: [row('old')], nextCursor: 'x' });
    await first;
    expect(list.items.value.map((item) => item.id)).toEqual(['new']);
    expect(list.hasMore.value).toBe(false);
    expect(list.loading.value).toBe(false);
  });

  it('drops a load-more that a refresh superseded', async () => {
    const more = deferred<PagedReply<Row>>();
    let calls = 0;
    const list = createPagedList<Row>({
      key: (item) => item.id,
      fetch: (after) => {
        calls += 1;
        if (after === 'c1') return more.promise;
        return Promise.resolve({ items: [row(calls === 1 ? 'a' : 'z')], nextCursor: calls === 1 ? 'c1' : null });
      },
    });
    await list.refresh();
    const pending = list.loadMore();
    await list.refresh();
    more.resolve({ items: [row('late')], nextCursor: null });
    await pending;
    expect(list.items.value.map((item) => item.id)).toEqual(['z']);
  });

  it('keeps the failure and the rows already shown, and a reset makes pending replies stale', async () => {
    const late = deferred<PagedReply<Row>>();
    const outcomes: Array<Promise<PagedReply<Row>>> = [Promise.resolve({ items: [row('a')], nextCursor: null }), Promise.reject(new Error('down')), late.promise];
    const list = createPagedList<Row>({ key: (item) => item.id, fetch: () => outcomes.shift()! });
    await list.refresh();
    await list.refresh();
    expect((list.error.value as Error).message).toBe('down');
    expect(list.items.value).toHaveLength(1);
    const pending = list.refresh();
    list.reset();
    late.resolve({ items: [row('private')], nextCursor: null });
    await pending;
    expect(list.items.value).toEqual([]);
    expect(list.loaded.value).toBe(false);
    expect(list.loading.value).toBe(false);
  });

  it('replaces and removes single rows', async () => {
    const list = createPagedList<Row>({ key: (item) => item.id, fetch: async () => ({ items: [row('a'), row('b')], nextCursor: null }) });
    await list.refresh();
    list.replace({ id: 'a', label: 'named' });
    expect(list.items.value[0]).toEqual({ id: 'a', label: 'named' });
    list.remove('b');
    expect(list.items.value.map((item) => item.id)).toEqual(['a']);
  });
});
