<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { routeLocationKey, routerKey } from 'vue-router';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import InstallPluginDialog from '@/features/install-plugin/ui/InstallPluginDialog.vue';
import type { PluginCatalogTab, PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import type { PluginCommand } from '@/entities/plugin/lib/plugin-menu.ts';
import { parseRemoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useProcessorsStore } from '@/entities/processor/model/processors-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import { usePluginSyncAdapter } from '@/features/manage-plugins/model/use-plugin-sync-adapter.ts';
import SyncButton from '@/features/sync-changes/ui/SyncButton.vue';
import PluginCatalog from '@/widgets/plugin-catalog/ui/PluginCatalog.vue';
import type { PluginDetailsRequest, PluginDetailsTab } from '@/widgets/plugin-details/model/details-tab.ts';
import PluginDetails from '@/widgets/plugin-details/ui/PluginDetails.vue';
import PluginJobsPanel from '@/widgets/plugin-jobs/ui/PluginJobsPanel.vue';
import PluginPreferences from '@/widgets/plugin-preferences/ui/PluginPreferences.vue';
import ProcessorDetails from '@/widgets/plugin-processors/ui/ProcessorDetails.vue';
import ProcessorsPanel from '@/widgets/plugin-processors/ui/ProcessorsPanel.vue';
import RemotePluginDetails from '@/widgets/plugin-remote-details/ui/RemotePluginDetails.vue';

type PluginsView = 'plugins' | 'processors';

const plugins = usePluginControls();
const syncAdapter = usePluginSyncAdapter();
const packages = usePackagesStore();
const processorsStore = useProcessorsStore();
const capabilities = useCapabilityStore();
const saved = loadViewState().plugins;

/**
 * The installed plugins are the entry point. A saved `catalog` or `cloud` view (the removed Cloud
 * catalog pages) falls back to it; only processors count as an explicit choice.
 */
function restoreView(state: typeof saved): PluginsView | null {
  return state?.view === 'processors' ? 'processors' : null;
}
const view = ref<PluginsView | null>(restoreView(saved));
const catalog = ref<PluginCatalogTab>(saved?.catalog ?? 'installed');
const query = ref(saved?.query ?? '');
const processorQuery = ref('');
const selected = ref<string | null>(saved?.selected ?? null);
const detailsTab = ref<PluginDetailsTab>(saved?.details ?? 'general');
/** Key of the open processor; like the open plugin, it replaces the list inside the same panel. */
const processorSelected = ref<string | null>(null);
/** Plugin-wide preferences replace the catalog like a detail does; `?view=preferences` opens them directly. */
const route = inject(routeLocationKey, null);
const router = inject(routerKey, null);
const preferencesOpen = ref(route?.query.view === 'preferences');

function openPlugin(id: string): void {
  selected.value = id;
}

/** Handed to the details panel, which owns the uninstall dialog and test result. */
const detailsRequest = ref<PluginDetailsRequest | null>(null);

function runCommand(id: string, command: PluginCommand): void {
  const plugin = behavior.value.plugins.find((entry) => entry.descriptor.id === id);
  if (!plugin) return;
  switch (command) {
    case 'toggle-enabled':
      plugins.setEnabled(id, !plugin.enabled);
      return;
    case 'install':
      plugins.setInstalled(id, true);
      return;
    case 'remove':
      plugins.setInstalled(id, false);
      return;
    case 'settings':
    case 'connection':
    case 'versions':
      openPlugin(id);
      // After the selection watcher, which resets the tab to General.
      void nextTick(() => { detailsTab.value = command; });
      return;
    case 'uninstall':
    case 'test':
      openPlugin(id);
      detailsRequest.value = command;
      return;
    default:
      openPlugin(id);
  }
}

const behavior = computed(() => plugins.behavior.value);
const activeView = computed<PluginsView>(() => view.value ?? 'plugins');
/** The panel's tabs: processors are a view of their own, the others are the catalog tabs of the plugins view. */
const panelTab = computed<PluginsPanelTab>({
  get: () => (activeView.value === 'processors' ? 'processors' : catalog.value),
  set: (next) => {
    if (next === 'processors') {
      view.value = 'processors';
      return;
    }
    view.value = 'plugins';
    catalog.value = next;
  },
});
const selectedPlugin = computed(
  () => behavior.value.plugins.find((plugin) => plugin.descriptor.id === selected.value) ?? null,
);
const selectedRemote = computed(() => parseRemoteSelection(selected.value));
/** The open plugin replaces the catalog in the panel; closing it is the back arrow. */
const detailsOpen = computed(() => activeView.value === 'plugins' && Boolean(selectedPlugin.value || selectedRemote.value));

function closePreferences(): void {
  preferencesOpen.value = false;
  if (route?.query.view) void router?.replace({ query: {} });
}

function closeDetails(): void {
  selected.value = null;
  detailsRequest.value = null;
}
const updateEntries = computed(() => packages.updates?.updates ?? []);
const remoteEntries = computed(() => packages.searchResults?.entries ?? []);
/**
 * Explore lists the Cloud catalog unless the host says it does not serve it: `undefined`
 * (discovery pending) keeps calling, `false` falls back to the registry search of older hosts.
 */
const exploreCatalog = computed(() => capabilities.supports('catalog.items') !== false);
/** Registry fallback only: unknown until the registry list loads; only `false` shows the unavailable state. */
const marketplaceAvailable = computed(() =>
  packages.registriesLoaded ? packages.registries.some((registry) => registry.enabled) : undefined,
);

function search(): void {
  void packages.search();
}

function openCatalogTab(next: PluginCatalogTab): void {
  if (next === 'browse' && !exploreCatalog.value && !packages.searchResults && !packages.searching) search();
}

/** Older hosts: the registry list and its search back Explore. */
function loadRegistryExplore(): void {
  void packages.loadRegistries();
  openCatalogTab(catalog.value);
}

onMounted(() => {
  void packages.initialize();
  if (!exploreCatalog.value) loadRegistryExplore();
});

watch(catalog, openCatalogTab);
watch(exploreCatalog, (on) => { if (!on) loadRegistryExplore(); });

watch(selected, (next, previous) => {
  if (next !== null && next !== previous) detailsTab.value = 'general';
});

watch(
  () => selectedPlugin.value?.installed,
  (installed) => {
    if (installed === undefined) return;
    const tab: PluginCatalogTab = installed ? 'installed' : 'browse';
    if (catalog.value !== tab && !query.value.trim()) catalog.value = tab;
  },
);

function persistView(): void {
  saveViewState('plugins', {
    view: view.value ?? undefined,
    catalog: catalog.value,
    query: query.value,
    selected: selected.value ?? undefined,
    details: detailsTab.value,
  });
}
const persist = debounce(persistView);
watch([view, catalog, query, selected, detailsTab], persist);
onBeforeUnmount(() => {
  persist.cancel();
  persistView();
});
// The list the processor was opened from may no longer hold it.
watch(activeView, () => { processorSelected.value = null; });
</script>

<template>
  <div
    class="plugins-page"
    data-guide-scope="plugins"
  >
    <p
      v-if="plugins.behaviorError.value"
      class="plugins-page__banner plugins-page__banner--error"
      role="alert"
    >
      {{ plugins.behaviorError.value }}
    </p>
    <BaseAlert
      v-if="packages.updatesError"
      :title="packages.updatesError"
      compact
    />
    <PluginJobsPanel />

    <!-- One navigation area: the catalog or processors, or the open item with a way back. -->
    <PluginPreferences
      v-if="preferencesOpen"
      @back="closePreferences"
    />
    <PluginDetails
      v-else-if="detailsOpen && selectedPlugin"
      :key="selectedPlugin.descriptor.id"
      v-model:tab="detailsTab"
      v-model:request="detailsRequest"
      :plugin="selectedPlugin"
      :actions="behavior.actions"
      :action-types="behavior.actionTypes"
      @back="closeDetails"
    />
    <RemotePluginDetails
      v-else-if="detailsOpen && selectedRemote"
      :key="selected ?? ''"
      :registry="selectedRemote.registry"
      :plugin-id="selectedRemote.id"
      @back="closeDetails"
    />
    <ProcessorDetails
      v-else-if="activeView === 'processors' && processorSelected"
      :selected="processorSelected"
      :plugins="behavior.plugins"
      @back="processorSelected = null"
    />
    <PluginCatalog
      v-else
      v-model:tab="panelTab"
      v-model:query="query"
      v-model:remote-query="packages.searchQuery"
      v-model:processor-query="processorQuery"
      data-guide="plugin-catalog"
      :plugins="behavior.plugins"
      :action-types="behavior.actionTypes"
      :remote="remoteEntries"
      :updates="updateEntries"
      :searching="packages.searching"
      :search-error="packages.searchError"
      :checking-updates="packages.checkingUpdates"
      :marketplace-available="marketplaceAvailable"
      :explore-catalog="exploreCatalog"
      :install-disabled="packages.review.open"
      @install-package="plugins.installPackage()"
      @search="search"
      @check-updates="packages.checkUpdates()"
      @update="(id, version) => packages.updatePlugin(id, version)"
      @update-all="packages.updateAll()"
      @refresh-processors="processorsStore.refresh()"
      @preferences="preferencesOpen = true"
      @open="openPlugin"
      @command="runCommand"
    >
      <template #tools>
        <SyncButton
          :adapter="syncAdapter"
          size="sm"
        />
      </template>
      <template #processors>
        <ProcessorsPanel
          :plugins="behavior.plugins"
          :query="processorQuery"
          @open="processorSelected = $event"
        />
      </template>
    </PluginCatalog>
    <InstallPluginDialog />
  </div>
</template>

<style scoped>
.plugins-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}




.plugins-page__banner {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.plugins-page__banner--error {
  border-color: var(--color-danger);
  color: var(--color-danger-hover);
}

@media (width < 900px) {
  .plugins-page {
    padding: var(--space-3);
  }
}
</style>
