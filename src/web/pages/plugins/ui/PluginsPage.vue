<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import InstallPluginDialog from '@/features/install-plugin/ui/InstallPluginDialog.vue';
import type { PluginCatalogTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import { parseRemoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseSplitter from '@/shared/ui/splitter/BaseSplitter.vue';
import PluginCatalog from '@/widgets/plugin-catalog/ui/PluginCatalog.vue';
import type { PluginDetailsTab } from '@/widgets/plugin-details/model/details-tab.ts';
import PluginDetails from '@/widgets/plugin-details/ui/PluginDetails.vue';
import PluginJobsPanel from '@/widgets/plugin-jobs/ui/PluginJobsPanel.vue';
import ProcessorsPanel from '@/widgets/plugin-processors/ui/ProcessorsPanel.vue';
import RemotePluginDetails from '@/widgets/plugin-remote-details/ui/RemotePluginDetails.vue';

type PluginsView = 'plugins' | 'processors';

const { t } = useI18n();
const plugins = usePluginControls();
const packages = usePackagesStore();
const saved = loadViewState().plugins;

const view = ref<PluginsView>(saved?.view ?? 'plugins');
const catalog = ref<PluginCatalogTab>(saved?.catalog ?? 'installed');
const query = ref(saved?.query ?? '');
const processorQuery = ref('');
const selected = ref<string | null>(saved?.selected ?? null);
const detailsTab = ref<PluginDetailsTab>(saved?.details ?? 'general');
const catalogWidth = ref(300);
const isNarrow = useMediaQuery('(width < 900px)');
const mobileDetailsOpen = ref(false);

function openPlugin(id: string): void {
  selected.value = id;
  if (isNarrow.value) mobileDetailsOpen.value = true;
}

const behavior = computed(() => plugins.behavior.value);
const selectedPlugin = computed(
  () => behavior.value.plugins.find((plugin) => plugin.descriptor.id === selected.value) ?? null,
);
const selectedRemote = computed(() => parseRemoteSelection(selected.value));
const updateEntries = computed(() => packages.updates?.updates ?? []);
const remoteEntries = computed(() => packages.searchResults?.entries ?? []);

function search(): void {
  void packages.search();
}

function openCatalogTab(next: PluginCatalogTab): void {
  if (next === 'browse' && !packages.searchResults && !packages.searching) search();
}

onMounted(() => {
  void packages.initialize();
  openCatalogTab(catalog.value);
});

watch(catalog, openCatalogTab);
const views = computed(() => [
  { value: 'plugins' as const, label: t('plugins.views.plugins') },
  { value: 'processors' as const, label: t('plugins.views.processors') },
]);

watch(selected, (next, previous) => {
  if (previous !== null && next !== previous) detailsTab.value = 'general';
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
    view: view.value,
    catalog: catalog.value,
    query: query.value,
    selected: selected.value ?? undefined,
    details: detailsTab.value,
  });
});
watch([view, catalog, query, selected, detailsTab], persist);
</script>

<template>
  <div class="plugins-page">
    <header class="plugins-page__toolbar">
      <BaseSegmented
        v-model="view"
        :options="views"
        :aria-label="t('plugins.title')"
      />
      <span class="plugins-page__lead">{{ t('plugins.lead') }}</span>
      <BaseButton
        variant="primary"
        :disabled="packages.review.open"
        @click="plugins.installPackage()"
      >
        <template #icon>
          <IconPlus />
        </template>
        {{ t('plugins.install.package') }}
      </BaseButton>
    </header>

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

    <BaseSplitter
      v-if="view === 'plugins' && !isNarrow"
      v-model:size="catalogWidth"
      class="plugins-page__workspace"
      :orientation="isNarrow ? 'vertical' : 'horizontal'"
      :min="isNarrow ? 200 : 240"
      :max="520"
    >
      <template #start>
        <PluginCatalog
          v-model:tab="catalog"
          v-model:query="query"
          v-model:remote-query="packages.searchQuery"
          v-model:selected="selected"
          :plugins="behavior.plugins"
          :action-types="behavior.actionTypes"
          :remote="remoteEntries"
          :updates="updateEntries"
          :searching="packages.searching"
          :search-error="packages.searchError"
          :checking-updates="packages.checkingUpdates"
          @install-package="plugins.installPackage()"
          @install-spec="plugins.installSpec"
          @search="search"
          @check-updates="packages.checkUpdates()"
          @update="(id, version) => packages.updatePlugin(id, version)"
          @update-all="packages.updateAll()"
          @open="openPlugin"
        />
      </template>
      <template #end>
        <PluginDetails
          v-if="selectedPlugin"
          :key="selectedPlugin.descriptor.id"
          v-model:tab="detailsTab"
          :plugin="selectedPlugin"
          :actions="behavior.actions"
          :action-types="behavior.actionTypes"
        />
        <RemotePluginDetails
          v-else-if="selectedRemote"
          :key="selected ?? ''"
          :registry="selectedRemote.registry"
          :plugin-id="selectedRemote.id"
        />
        <div
          v-else
          class="plugins-page__placeholder"
        >
          <BaseEmptyState :title="t('plugins.details.empty')">
            <template #icon>
              <IconPuzzle :size="28" />
            </template>
          </BaseEmptyState>
        </div>
      </template>
    </BaseSplitter>
    <div
      v-else-if="view === 'plugins'"
      class="plugins-page__mobile"
    >
      <template v-if="mobileDetailsOpen && (selectedPlugin || selectedRemote)">
        <BaseButton
          class="plugins-page__back"
          @click="mobileDetailsOpen = false"
        >
          ← {{ t('common.back') }}
        </BaseButton>
        <PluginDetails
          v-if="selectedPlugin"
          :key="selectedPlugin.descriptor.id"
          v-model:tab="detailsTab"
          :plugin="selectedPlugin"
          :actions="behavior.actions"
          :action-types="behavior.actionTypes"
        />
        <RemotePluginDetails
          v-else-if="selectedRemote"
          :key="selected ?? ''"
          :registry="selectedRemote.registry"
          :plugin-id="selectedRemote.id"
        />
      </template>
      <PluginCatalog
        v-else
        v-model:tab="catalog"
        v-model:query="query"
        v-model:remote-query="packages.searchQuery"
        v-model:selected="selected"
        :plugins="behavior.plugins"
        :action-types="behavior.actionTypes"
        :remote="remoteEntries"
        :updates="updateEntries"
        :searching="packages.searching"
        :search-error="packages.searchError"
        :checking-updates="packages.checkingUpdates"
        @install-package="plugins.installPackage()"
        @install-spec="plugins.installSpec"
        @search="search"
        @check-updates="packages.checkUpdates()"
        @update="(id, version) => packages.updatePlugin(id, version)"
        @update-all="packages.updateAll()"
        @open="openPlugin"
      />
    </div>
    <ProcessorsPanel
      v-else
      v-model:query="processorQuery"
      :plugins="behavior.plugins"
    />
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

.plugins-page__toolbar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-5);
}

.plugins-page__lead {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
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

.plugins-page__workspace {
  flex: 1;
}

.plugins-page__mobile {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-3);
  min-height: 0;
}

.plugins-page__back {
  align-self: flex-start;
  flex: none;
}

.plugins-page__mobile > :deep(.plugin-details) {
  flex: 1;
  min-height: 0;
}

.plugins-page__placeholder {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

@media (width < 900px) {
  .plugins-page {
    padding: var(--space-3);
  }

  .plugins-page__toolbar {
    flex-wrap: wrap;
    row-gap: var(--space-2);
  }

  .plugins-page__lead {
    display: none;
  }

  .plugins-page__workspace {
    flex: none;
    min-height: 0;
  }
}
</style>
