<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import InstallPluginDialog from '@/features/install-plugin/ui/InstallPluginDialog.vue';
import type { PluginCatalogTab, PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import type { PluginCommand } from '@/entities/plugin/lib/plugin-menu.ts';
import { parseRemoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useProcessorsStore } from '@/entities/processor/model/processors-store.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import CloudPluginCatalog from '@/widgets/cloud-plugin-catalog/ui/CloudPluginCatalog.vue';
import UnifiedCatalog from '@/widgets/unified-catalog/ui/UnifiedCatalog.vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import PluginCatalog from '@/widgets/plugin-catalog/ui/PluginCatalog.vue';
import type { PluginDetailsRequest, PluginDetailsTab } from '@/widgets/plugin-details/model/details-tab.ts';
import PluginDetails from '@/widgets/plugin-details/ui/PluginDetails.vue';
import PluginJobsPanel from '@/widgets/plugin-jobs/ui/PluginJobsPanel.vue';
import ProcessorDetails from '@/widgets/plugin-processors/ui/ProcessorDetails.vue';
import ProcessorsPanel from '@/widgets/plugin-processors/ui/ProcessorsPanel.vue';
import RemotePluginDetails from '@/widgets/plugin-remote-details/ui/RemotePluginDetails.vue';

type PluginsView = 'plugins' | 'processors' | 'catalog';

const { t } = useI18n();
const plugins = usePluginControls();
const packages = usePackagesStore();
const processorsStore = useProcessorsStore();
const saved = loadViewState().plugins;

/**
 * `null` means no explicit choice: the catalog on hosts that announce it, the installed
 * plugins otherwise. The old `cloud` view is the catalog. Before the catalog was the entry
 * point, `plugins` was the saved default, so it only counts as a choice with a plugin open.
 */
function restoreView(state: typeof saved): PluginsView | null {
  if (!state?.view) return null;
  if (state.view === 'cloud') return 'catalog';
  if (state.view === 'plugins' && !state.selected) return null;
  return state.view;
}
const view = ref<PluginsView | null>(restoreView(saved));
const catalog = ref<PluginCatalogTab>(saved?.catalog ?? 'installed');
const query = ref(saved?.query ?? '');
const processorQuery = ref('');
const selected = ref<string | null>(saved?.selected ?? null);
const detailsTab = ref<PluginDetailsTab>(saved?.details ?? 'general');
/** Key of the open processor; like the open plugin, it replaces the list inside the same panel. */
const processorSelected = ref<string | null>(null);

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
const installedIds = computed(() => new Set(behavior.value.plugins.map((plugin) => plugin.descriptor.id)));
// The unified catalog needs the host to announce catalog.items; otherwise the legacy Cloud view stays.
const capabilities = useCapabilityStore();
const unifiedCatalog = computed(() => capabilities.supports('catalog.items') === true);
const activeView = computed<PluginsView>(() => view.value ?? (unifiedCatalog.value ? 'catalog' : 'plugins'));
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
/** A plugin chosen in the catalog opens in the installed-plugins workspace. */
function openInstalled(id: string): void {
  view.value = 'plugins';
  catalog.value = 'installed';
  openPlugin(id);
}
const selectedPlugin = computed(
  () => behavior.value.plugins.find((plugin) => plugin.descriptor.id === selected.value) ?? null,
);
const selectedRemote = computed(() => parseRemoteSelection(selected.value));
/** The open plugin replaces the catalog in the panel; closing it is the back arrow. */
const detailsOpen = computed(() => activeView.value === 'plugins' && Boolean(selectedPlugin.value || selectedRemote.value));

function closeDetails(): void {
  selected.value = null;
  detailsRequest.value = null;
}
const updateEntries = computed(() => packages.updates?.updates ?? []);
const remoteEntries = computed(() => packages.searchResults?.entries ?? []);
/** Unknown until the registry list loads; only `false` shows the unavailable state. */
const marketplaceAvailable = computed(() =>
  packages.registriesLoaded ? packages.registries.some((registry) => registry.enabled) : undefined,
);

function search(): void {
  void packages.search();
}

function openCatalogTab(next: PluginCatalogTab): void {
  if (next === 'browse' && !packages.searchResults && !packages.searching) search();
}

onMounted(() => {
  void packages.initialize();
  void packages.loadRegistries();
  openCatalogTab(catalog.value);
});

watch(catalog, openCatalogTab);

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

const persist = debounce(() => {
  saveViewState('plugins', {
    view: view.value ?? undefined,
    catalog: catalog.value,
    query: query.value,
    selected: selected.value ?? undefined,
    details: detailsTab.value,
  });
});
watch([view, catalog, query, selected, detailsTab], persist);
// The list the processor was opened from may no longer hold it.
watch(activeView, () => { processorSelected.value = null; });
</script>

<template>
  <div class="plugins-page">
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
    <template v-if="activeView === 'catalog'">
      <template v-if="unifiedCatalog">
        <h2 class="plugins-page__sr-only">
          {{ t('plugins.views.back') }}
        </h2>
        <UnifiedCatalog
          :installed-ids="installedIds"
          @manage-processors="view = 'processors'"
          @manage-installed="view = 'plugins'"
          @install-package="plugins.installPackage()"
          @open-plugin="openInstalled"
        />
      </template>
      <template v-else>
        <div class="plugins-page__bar">
          <BaseIconButton
            :label="t('plugins.views.plugins')"
            variant="default"
            size="sm"
            @click="view = 'plugins'"
          >
            <IconChevronLeft />
          </BaseIconButton>
          <BaseIconButton
            class="plugins-page__install"
            :label="t('plugins.install.package')"
            variant="primary"
            size="sm"
            :disabled="packages.review.open"
            @click="plugins.installPackage()"
          >
            <IconPlus />
          </BaseIconButton>
        </div>
        <CloudPluginCatalog />
      </template>
    </template>
    <template v-else>
      <PluginDetails
        v-if="detailsOpen && selectedPlugin"
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
        :plugins="behavior.plugins"
        :action-types="behavior.actionTypes"
        :remote="remoteEntries"
        :updates="updateEntries"
        :searching="packages.searching"
        :search-error="packages.searchError"
        :checking-updates="packages.checkingUpdates"
        :marketplace-available="marketplaceAvailable"
        :install-disabled="packages.review.open"
        :catalog-label="unifiedCatalog ? t('plugins.views.back') : t('plugins.views.cloud')"
        @install-package="plugins.installPackage()"
        @install-spec="plugins.installSpec"
        @search="search"
        @check-updates="packages.checkUpdates()"
        @update="(id, version) => packages.updatePlugin(id, version)"
        @update-all="packages.updateAll()"
        @refresh-processors="processorsStore.refresh()"
        @open-catalog="view = 'catalog'"
        @open="openPlugin"
        @command="runCommand"
      >
        <template #processors>
          <ProcessorsPanel
            :plugins="behavior.plugins"
            :query="processorQuery"
            @open="processorSelected = $event"
          />
        </template>
      </PluginCatalog>
    </template>
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

.plugins-page__sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.plugins-page__bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
}

.plugins-page__install {
  margin-left: auto;
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
