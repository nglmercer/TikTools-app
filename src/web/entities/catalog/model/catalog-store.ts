import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { parseRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
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
 * guard, so a stale reply never replaces a newer query. "Instalados" is resolved from
 * the local inventory with bounded lookups, not by filtering one public page.
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
  const error = ref<string | null>(null);
  const detail = ref<CatalogItemSummary | null>(null);
  const importState = ref<'idle' | 'working' | 'staged' | 'signIn' | 'invalid' | 'failed'>('idle');
  const importMessage = ref('');
  const account = useAccountStore();
  let installedTicket = 0;

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
    const generation = search.begin(query);
    if (!append) publish();
    loading.value = true;
    error.value = null;
    try {
      const page = mine
        ? await fetchMinePage(client, query, after)
        : await fetchCatalogPage(client, query, after);
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

  /** Resolves the installed plugins with bounded `catalog.item` lookups. */
  async function refreshInstalled(): Promise<void> {
    const ticket = ++installedTicket;
    const ids = [...installedIds.value].sort();
    installedLimited.value = ids.length > MAX_INSTALLED_LOOKUPS;
    loading.value = true;
    error.value = null;
    try {
      const entries = await mapBounded(ids.slice(0, MAX_INSTALLED_LOOKUPS), LOOKUP_CONCURRENCY, (id) =>
        resolveInstalledPlugin(client, id, target.value),
      );
      if (ticket === installedTicket) installedEntries.value = entries;
    } catch (failure) {
      if (ticket === installedTicket) error.value = errorMessage(failure);
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
    detail.value = item;
    if (!item.published) return;
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
   * Imports a published or owned template through the local library journal, the
   * same path as the Cloud library. The imported template stays inert until the user acts.
   */
  async function importTemplate(item: CatalogItemSummary): Promise<boolean> {
    if (item.kind !== 'template' || !item.published || item.revision === null || item.sha256 === null) return false;
    const origin = account.status?.origin ?? null;
    const accountId = account.status?.accountId ?? null;
    if (!origin || !accountId) {
      importState.value = 'signIn';
      return false;
    }
    importState.value = 'working';
    importMessage.value = '';
    try {
      const document = await client.call<{ document: unknown }>('cloudLibrary.read', {
        scope: filters.value.view === 'mine' ? 'mine' : 'public',
        id: item.id,
        revision: String(item.revision),
      });
      const parsed = parseRuleTemplate(document.document);
      if (!parsed.ok) {
        importState.value = 'invalid';
        return false;
      }
      await libraryJournal(client).request(
        { operation: 'import-templates', templates: [parsed.template] },
        null,
        { origin, accountId, documentId: item.id, revision: String(item.revision), sha256: item.sha256 },
      );
      importState.value = 'staged';
      return true;
    } catch (failure) {
      importState.value = 'failed';
      importMessage.value = errorMessage(failure);
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
    error,
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
    importTemplate,
    importState,
    importMessage,
  };
});
