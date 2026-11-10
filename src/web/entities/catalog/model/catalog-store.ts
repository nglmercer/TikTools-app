import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import {
  fetchCatalogItem,
  fetchCatalogPage,
  type CatalogItemSummary,
} from '../api/catalog-api.ts';
import {
  createCatalogSearch,
  DEFAULT_CATALOG_FILTERS,
  filtersToQuery,
  type CatalogFilters,
} from './catalog-query.ts';

/**
 * Unified catalog state: the current query, its accepted pages, and the
 * selected item. Every reply is applied through the generation guard in
 * catalog-query, so a slow older page cannot replace a newer result.
 */
export const useCatalogStore = defineStore('catalog', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const packages = usePackagesStore();
  const search = createCatalogSearch<CatalogItemSummary>();

  const filters = ref<CatalogFilters>({ ...DEFAULT_CATALOG_FILTERS });
  const target = ref<string | undefined>(undefined);
  const installedIds = ref<ReadonlySet<string>>(new Set());
  const items = ref<readonly CatalogItemSummary[]>([]);
  const cursor = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const detail = ref<CatalogItemSummary | null>(null);

  /** Whether this host announced the unified catalog. `undefined` is not `false`. */
  const available = computed(() => capabilities.supports('catalog.items'));
  /** "Installed" is resolved locally; the server page is not narrowed by it. */
  const visible = computed(() => {
    if (filters.value.view === 'installed') return items.value.filter((item) => installedIds.value.has(item.id));
    // "Mine" needs the signed-in library, which this host does not expose yet.
    if (filters.value.view === 'mine') return [];
    return items.value;
  });
  const mineUnavailable = computed(() => filters.value.view === 'mine');
  const hasMore = computed(() => cursor.value !== null);

  function publish(): void {
    items.value = search.items();
    cursor.value = search.cursor();
  }

  async function load(append: boolean): Promise<void> {
    if (available.value === false) return;
    const query = filtersToQuery(filters.value, target.value);
    const after = append ? search.cursor() : null;
    if (append && after === null) return;
    const generation = search.begin(query);
    if (!append) publish();
    loading.value = true;
    error.value = null;
    try {
      const page = await fetchCatalogPage(client, query, after);
      if (!search.isNewest(generation)) return;
      const accepted = append
        ? { items: [...search.items(), ...page.items], nextCursor: page.nextCursor }
        : page;
      if (search.settle(generation, accepted)) publish();
    } catch (failure) {
      if (search.isNewest(generation)) error.value = errorMessage(failure);
    } finally {
      if (search.isNewest(generation)) loading.value = false;
    }
  }

  /** Reloads the first page for the current filters. */
  function refresh(): Promise<void> {
    return load(false);
  }

  /** Appends the next page for the same query. No-op without a cursor. */
  function loadMore(): Promise<void> {
    if (loading.value) return Promise.resolve();
    return load(true);
  }

  function setFilters(next: CatalogFilters): Promise<void> {
    filters.value = next;
    return refresh();
  }

  /** Sets the host target and reloads the first page. Called on mount and on change. */
  function setTarget(next: string | undefined): Promise<void> {
    target.value = next;
    return refresh();
  }

  function setInstalled(ids: ReadonlySet<string>): void {
    installedIds.value = ids;
  }

  async function openDetail(item: CatalogItemSummary): Promise<void> {
    detail.value = item;
    try {
      const fresh = await fetchCatalogItem(client, item.kind, item.id, target.value);
      if (detail.value?.id === item.id && detail.value.kind === item.kind) detail.value = fresh;
    } catch (failure) {
      // The list summary stays visible; the failure is shown as an error for the list.
      error.value = errorMessage(failure);
    }
  }

  function closeDetail(): void {
    detail.value = null;
  }

  /**
   * Plugins install through the signed package review, the same path as any
   * other registry install. Templates and profiles have no install here.
   */
  function install(item: CatalogItemSummary): Promise<void> {
    if (item.kind !== 'plugin' || item.yanked) return Promise.resolve();
    return packages.reviewSpec({ kind: 'registry', id: item.id });
  }

  return {
    filters,
    target,
    items: visible,
    cursor,
    loading,
    error,
    detail,
    available,
    hasMore,
    mineUnavailable,
    refresh,
    loadMore,
    setFilters,
    setTarget,
    setInstalled,
    openDetail,
    closeDetail,
    install,
  };
});
