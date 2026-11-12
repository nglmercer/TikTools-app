import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useCloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import { useLibraryImport } from '@/entities/cloud-bundles/model/use-library-import.ts';
import { createRequestScope } from '@/shared/lib/request-scope.ts';
import {
  fetchCatalogItem,
  fetchCatalogPage,
  fetchMinePage,
  resolveInstalledPlugin,
  type CatalogItemSummary,
} from '../api/catalog-api.ts';
import {
  createCatalogSearch,
  DEFAULT_CATALOG_FILTERS,
  filtersToQuery,
  type CatalogDataState,
  type CatalogFilters,
} from './catalog-query.ts';

/** Installed plugins resolved per view. Beyond this the view says so instead of guessing. */
export const MAX_INSTALLED_LOOKUPS = 50;
const LOOKUP_CONCURRENCY = 4;

async function mapBounded<T, R>(items: readonly T[], limit: number, work: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (next < items.length) {
      const index = next;
      next += 1;
      results[index] = await work(items[index] as T);
    }
  });
  await Promise.all(workers);
  return results;
}

/**
 * Unified catalog state. "All" and "Míos" page through the server with a generation
 * guard, so a stale reply never replaces a newer query, and the two never share items or
 * cursors. "Instalados" is resolved from the local inventory with bounded lookups, not by
 * filtering one public page. A change of account, session or host drops what was loaded
 * (private items included) and makes every pending reply stale.
 */
export const useCatalogStore = defineStore('catalog', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const packages = usePackagesStore();
  const search = createCatalogSearch<CatalogItemSummary>();

  const filters = ref<CatalogFilters>({ ...DEFAULT_CATALOG_FILTERS });
  const target = ref<string | undefined>(undefined);
  const installedIds = ref<ReadonlySet<string>>(new Set());
  const installedEntries = ref<readonly CatalogItemSummary[]>([]);
  /** True when more installed plugins exist than the lookup budget covers. */
  const installedLimited = ref(false);
  const items = ref<readonly CatalogItemSummary[]>([]);
  const cursor = ref<string | null>(null);
  const loading = ref(false);
  /** Whether the last accepted page came from an empty plugin registry. */
  const dataState = ref<CatalogDataState>('ready');
  const error = ref<string | null>(null);
  const { detail: errorDetail, capture, clear: clearDetail } = useCloudFailure();
  const detail = ref<CatalogItemSummary | null>(null);
  const importState = ref<'idle' | 'working' | 'staged' | 'signIn' | 'failed'>('idle');
  const importMessage = ref('');
  const libraryImport = useLibraryImport();
  let installedTicket = 0;
  const detailScope = createRequestScope();
  const importScope = createRequestScope();

  /** A new account, session or host starts from nothing: no private data, no late reply. */
  const cloud = useCloudContext(() => {
    search.reset();
    installedTicket += 1;
    detailScope.invalidate();
    importScope.invalidate();
    items.value = [];
    cursor.value = null;
    dataState.value = 'ready';
    installedEntries.value = [];
    detail.value = null;
    loading.value = false;
    clearError();
    importState.value = 'idle';
    importMessage.value = '';
  });
  void cloud;

  function clearError(): void {
    error.value = null;
    clearDetail();
  }

  /** Whether this host announced the unified catalog. `undefined` is not `false`. */
  const available = computed(() => capabilities.supports('catalog.items'));
  const visible = computed(() =>
    filters.value.view === 'installed' ? installedEntries.value : items.value,
  );
  const hasMore = computed(() => filters.value.view !== 'installed' && cursor.value !== null);

  function publish(): void {
    items.value = search.items();
    cursor.value = search.cursor();
  }

  async function load(append: boolean): Promise<void> {
    if (available.value === false) return;
    const query = filtersToQuery(filters.value, target.value);
    const after = append ? search.cursor() : null;
    if (append && after === null) return;
    const mine = filters.value.view === 'mine';
    // A newer list request supersedes an installed lookup, so its end cannot clear this one's loading.
    installedTicket += 1;
    const generation = search.begin(query, mine ? 'mine' : 'public');
    if (!append) publish();
    loading.value = true;
    clearError();
    try {
      const page = mine
        ? await fetchMinePage(client, query, after)
        : await fetchCatalogPage(client, query, after);
      if (!search.isNewest(generation)) return;
      const accepted = append
        ? { items: [...search.items(), ...page.items], nextCursor: page.nextCursor }
        : page;
      if (search.settle(generation, accepted)) {
        publish();
        dataState.value = page.dataState ?? 'ready';
      }
    } catch (failure) {
      if (search.isNewest(generation)) error.value = capture(failure);
    } finally {
      if (search.isNewest(generation)) loading.value = false;
    }
  }

  /** Resolves the installed plugins with bounded `catalog.item` lookups. */
  async function refreshInstalled(): Promise<void> {
    const ticket = ++installedTicket;
    const ids = [...installedIds.value].sort();
    installedLimited.value = ids.length > MAX_INSTALLED_LOOKUPS;
    // The installed view supersedes any list request still in flight.
    search.reset();
    loading.value = true;
    clearError();
    try {
      const entries = await mapBounded(ids.slice(0, MAX_INSTALLED_LOOKUPS), LOOKUP_CONCURRENCY, (id) =>
        resolveInstalledPlugin(client, id, target.value),
      );
      if (ticket === installedTicket) installedEntries.value = entries;
    } catch (failure) {
      if (ticket === installedTicket) error.value = capture(failure);
    } finally {
      if (ticket === installedTicket) loading.value = false;
    }
  }

  function refreshCurrentView(): Promise<void> {
    if (filters.value.view === 'installed') return refreshInstalled();
    return load(false);
  }

  /** Reloads the current view from its first page. */
  function refresh(): Promise<void> {
    return refreshCurrentView();
  }

  /** Appends the next server page for the same query. No-op for the local installed view. */
  function loadMore(): Promise<void> {
    if (loading.value || filters.value.view === 'installed') return Promise.resolve();
    return load(true);
  }

  function setFilters(next: CatalogFilters): Promise<void> {
    filters.value = next;
    return refreshCurrentView();
  }

  /** Sets the host target and reloads the first page. Called on mount and on change. */
  function setTarget(next: string | undefined): Promise<void> {
    target.value = next;
    return refreshCurrentView();
  }

  function setInstalled(ids: ReadonlySet<string>): Promise<void> {
    installedIds.value = ids;
    return filters.value.view === 'installed' ? refreshInstalled() : Promise.resolve();
  }

  async function openDetail(item: CatalogItemSummary): Promise<void> {
    const ticket = detailScope.begin();
    detail.value = item;
    if (!item.published) return;
    try {
      const fresh = await fetchCatalogItem(client, item.kind, item.id, target.value);
      if (ticket.isCurrent() && detail.value?.id === item.id && detail.value.kind === item.kind) detail.value = fresh;
    } catch (failure) {
      // The list summary stays visible; the failure is shown as an error for the list.
      if (ticket.isCurrent()) error.value = capture(failure);
    }
  }

  function closeDetail(): void {
    detailScope.invalidate();
    detail.value = null;
  }

  /**
   * Imports a published or owned template or profile through the shared Cloud import
   * (the one owner-shell review, imported automations stay disabled). The result is
   * `true` only when the owner confirmed the review.
   */
  async function importItem(item: CatalogItemSummary): Promise<boolean> {
    if ((item.kind !== 'template' && item.kind !== 'profile') || !item.published || item.revision === null) return false;
    const ticket = importScope.begin();
    importState.value = 'working';
    importMessage.value = '';
    try {
      const source = { id: item.id, revision: item.revision };
      await (item.kind === 'template' ? libraryImport.importTemplate(source) : libraryImport.importProfile(source));
      if (ticket.isCurrent()) importState.value = 'staged';
      return ticket.isCurrent();
    } catch (failure) {
      if (!ticket.isCurrent()) return false;
      const code = failure instanceof ControlCallError ? failure.code : '';
      importState.value = code === 'signed_out' ? 'signIn' : code === 'cancelled' ? 'idle' : 'failed';
      importMessage.value = code === 'signed_out' || code === 'cancelled' ? '' : errorMessage(failure);
      return false;
    }
  }

  /**
   * Plugins install through the signed package review, the same path as any other
   * registry install. Templates and profiles are imported by their own flow.
   */
  function install(item: CatalogItemSummary): Promise<void> {
    if (item.kind !== 'plugin' || item.yanked || !item.published) return Promise.resolve();
    return packages.reviewSpec({ kind: 'registry', id: item.id });
  }

  return {
    filters,
    target,
    items: visible,
    installedLimited,
    cursor,
    loading,
    dataState,
    error,
    errorDetail,
    detail,
    available,
    hasMore,
    refresh,
    loadMore,
    setFilters,
    setTarget,
    setInstalled,
    openDetail,
    closeDetail,
    install,
    importItem,
    importState,
    importMessage,
  };
});
