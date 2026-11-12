<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { catalogPlugins, type PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import { isPluginCommand, pluginMenu, type PluginCommand } from '@/entities/plugin/lib/plugin-menu.ts';
import type { SearchEntry, UpdateInfo } from '@/entities/plugin-package/api/package-types.ts';
import { OFFICIAL_REGISTRY, remoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import ExploreList from '@/features/explore-catalog/ui/ExploreList.vue';
import ExploreToolbar from '@/features/explore-catalog/ui/ExploreToolbar.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconCompass from '@/shared/ui/icon/icons/IconCompass.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BaseContextMenu from '@/shared/ui/menu/BaseContextMenu.vue';
import type { MenuAction } from '@/shared/ui/menu/menu-types.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

type CatalogRow =
  | { key: string; kind: 'local'; plugin: PluginStatus }
  | { key: string; kind: 'remote'; entry: SearchEntry }
  | { key: string; kind: 'update'; update: UpdateInfo; plugin: PluginStatus | undefined };

const props = defineProps<{
  plugins: readonly PluginStatus[];
  actionTypes: readonly ActionTypeDefinition[];
  remote: readonly SearchEntry[];
  updates: readonly UpdateInfo[];
  searching?: boolean;
  searchError?: string;
  checkingUpdates?: boolean;
  /** False when no enabled registry exists (official one unpublished). Only the registry-based Explore reads it. */
  marketplaceAvailable?: boolean;
  /**
   * Explore lists the Cloud catalog (`catalog.items`) instead of the registry search. Hosts
   * that do not announce it keep the registry-based list.
   */
  exploreCatalog?: boolean;
  /** The package review is open: a second .plugin cannot be chosen yet. */
  installDisabled?: boolean;
}>();

const emit = defineEmits<{
  'install-package': [];
  search: [];
  'check-updates': [];
  update: [id: string, version?: string];
  'update-all': [];
  'refresh-processors': [];
  preferences: [];
  open: [key: string];
  command: [id: string, command: PluginCommand];
}>();

const tab = defineModel<PluginsPanelTab>('tab', { required: true });
const query = defineModel<string>('query', { default: '' });
const remoteQuery = defineModel<string>('remoteQuery', { default: '' });
const processorQuery = defineModel<string>('processorQuery', { default: '' });

const { t } = useI18n();
const titleId = useId();
const catalogStore = useCatalogStore();
/** Key of the row a context menu was opened on; never the open plugin, which has its own panel. */
const menuKey = ref<string | null>(null);

const installedCount = computed(() => props.plugins.filter((plugin) => plugin.installed).length);
const pendingUpdates = computed(() => props.updates.filter((entry) => entry.latest && entry.latestStatus === 'approved' && !entry.pinned));
const tabs = computed(() => [
  { value: 'browse' as const, label: t('plugins.catalog.browse'), icon: IconCompass },
  { value: 'installed' as const, label: t('plugins.catalog.installed'), icon: IconPuzzle, badge: installedCount.value },
  {
    value: 'updates' as const,
    label: t('plugins.catalog.updates'),
    icon: IconUpload,
    badge: pendingUpdates.value.length || undefined,
  },
  { value: 'processors' as const, label: t('plugins.views.processors'), icon: IconFlow },
]);

function updateName(update: UpdateInfo): string {
  const plugin = props.plugins.find((entry) => entry.descriptor.id === update.id);
  return plugin ? localizedText(plugin.descriptor.name) : update.id;
}

/** Explore shows the Cloud catalog only when the host serves it. */
const exploring = computed(() => props.exploreCatalog === true && tab.value === 'browse');
const installedIds = computed(() => new Set(props.plugins.filter((plugin) => plugin.installed).map((plugin) => plugin.descriptor.id)));
/** Bundled plugins already listed above the catalog: the catalog does not repeat them. */
const listedLocally = computed(() => new Set(props.plugins.filter((plugin) => !plugin.installed).map((plugin) => plugin.descriptor.id)));

const rows = computed<CatalogRow[]>(() => {
  if (tab.value === 'updates') {
    const words = query.value.trim().toLowerCase();
    return props.updates
      .filter((update) => !words || `${update.id} ${updateName(update)}`.toLowerCase().includes(words))
      .map((update) => ({
        key: update.id,
        kind: 'update' as const,
        update,
        plugin: props.plugins.find((plugin) => plugin.descriptor.id === update.id),
      }));
  }
  if (exploring.value) {
    // Bundled plugins that are not installed yet stay here; the Cloud catalog lists the rest below them.
    const { type, view, text } = catalogStore.filters;
    if (view === 'mine' || (type !== 'all' && type !== 'plugin')) return [];
    return catalogPlugins(props.plugins, 'browse', props.actionTypes, text, localizedText).map((plugin) => ({
      key: plugin.descriptor.id,
      kind: 'local' as const,
      plugin,
    }));
  }
  const local = catalogPlugins(props.plugins, tab.value === 'installed' ? 'installed' : 'browse', props.actionTypes, query.value, localizedText).map(
    (plugin) => ({ key: plugin.descriptor.id, kind: 'local' as const, plugin }),
  );
  if (tab.value === 'installed') return local;
  const known = new Set(props.plugins.map((plugin) => plugin.descriptor.id));
  const remote = props.remote
    .filter((entry) => !known.has(entry.id))
    .map((entry) => ({ key: remoteSelection(entry.registry, entry.id), kind: 'remote' as const, entry }));
  return [...local, ...remote];
});

/** Right-click (or Menu / Shift+F10) on an installed or bundled plugin row. */
const menuPlugin = computed(() => {
  const row = rows.value.find((entry) => entry.key === menuKey.value);
  return row?.kind === 'local' ? row.plugin : null;
});
const menuItems = computed(() => (menuPlugin.value ? pluginMenu(menuPlugin.value, props.actionTypes, t) : []));

function onMenuOpen(event: MouseEvent | KeyboardEvent): void {
  const row = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-plugin-key]');
  menuKey.value = row?.dataset['pluginKey'] ?? null;
}

function onMenu(action: MenuAction): void {
  const plugin = menuPlugin.value;
  if (plugin && isPluginCommand(action.id)) emit('command', plugin.descriptor.id, action.id);
}

const filtering = computed(() => query.value.trim().length > 0);
const searchable = computed(() => !(tab.value === 'browse' && !exploring.value && props.marketplaceAvailable === false));
const searchValue = computed({
  get: () => (tab.value === 'browse' ? remoteQuery.value : tab.value === 'processors' ? processorQuery.value : query.value),
  set: (value: string) => {
    if (tab.value === 'browse') remoteQuery.value = value;
    else if (tab.value === 'processors') processorQuery.value = value;
    else query.value = value;
  },
});
const searchLabel = computed(() =>
  tab.value === 'browse'
    ? t('plugins.catalog.searchRegistry')
    : tab.value === 'processors'
      ? t('plugins.processors.search')
      : t('plugins.catalog.search'),
);

function rowName(row: CatalogRow): string {
  if (row.kind === 'local') return localizedText(row.plugin.descriptor.name);
  if (row.kind === 'remote') return row.entry.name;
  return updateName(row.update);
}

function sourceLabel(plugin: PluginStatus): string {
  return t(`plugins.details.sources.${plugin.descriptor.source ?? 'builtin'}`);
}

function rowSub(row: CatalogRow): string {
  if (row.kind === 'remote') return row.entry.publisher ?? row.entry.registry;
  if (row.kind === 'update') return row.update.registry;
  return tab.value === 'installed' ? `${sourceLabel(row.plugin)} · ${row.plugin.descriptor.version}` : sourceLabel(row.plugin);
}

function submitSearch(): void {
  if (tab.value === 'browse') emit('search');
}

/** A catalog plugin opens its own details when installed, otherwise the details of the Cloud release. */
function openCatalogPlugin(id: string, installed: boolean): void {
  emit('open', installed ? id : remoteSelection(OFFICIAL_REGISTRY, id));
}
</script>

<template>
  <section
    class="plugin-catalog"
    :aria-labelledby="titleId"
  >
    <h2
      :id="titleId"
      class="plugin-catalog__title"
    >
      {{ t('plugins.title') }}
    </h2>
    <BaseTabs
      v-model="tab"
      class="plugin-catalog__tabs"
      :tabs="tabs"
    >
      <template #bar>
        <div class="plugin-catalog__bar">
          <ExploreToolbar v-if="exploring" />
          <form
            v-else-if="searchable"
            class="plugin-catalog__search"
            role="search"
            @submit.prevent="submitSearch"
          >
            <BaseTextField
              v-model="searchValue"
              type="search"
              size="sm"
              :placeholder="searchLabel"
              :aria-label="searchLabel"
            >
              <template #prefix>
                <IconSearch />
              </template>
            </BaseTextField>
          </form>
          <span class="plugin-catalog__tools">
            <slot name="tools" />
            <template v-if="tab === 'updates'">
              <BaseIconButton
                :label="checkingUpdates ? t('plugins.catalog.checking') : t('plugins.catalog.checkUpdates')"
                :hint="t('packages.liveStatus.hint')"
                variant="default"
                size="sm"
                :disabled="checkingUpdates"
                @click="emit('check-updates')"
              >
                <IconRefresh />
              </BaseIconButton>
              <BaseIconButton
                :label="t('plugins.catalog.updateAll')"
                variant="default"
                size="sm"
                :disabled="checkingUpdates || !pendingUpdates.length"
                @click="emit('update-all')"
              >
                <IconUpload />
              </BaseIconButton>
            </template>
            <BaseIconButton
              v-else-if="tab === 'processors'"
              :label="t('plugins.processors.refresh')"
              variant="default"
              size="sm"
              @click="emit('refresh-processors')"
            >
              <IconRefresh />
            </BaseIconButton>
            <BaseIconButton
              :label="t('plugins.catalog.preferences')"
              variant="default"
              size="sm"
              @click="emit('preferences')"
            >
              <IconGear />
            </BaseIconButton>
            <BaseIconButton
              :label="t('plugins.install.package')"
              variant="primary"
              size="sm"
              :disabled="installDisabled"
              @click="emit('install-package')"
            >
              <IconPlus />
            </BaseIconButton>
          </span>
        </div>
      </template>

      <div class="plugin-catalog__body">
        <p
          v-if="tab === 'browse' && !exploring && marketplaceAvailable === false"
          class="plugin-catalog__notice"
          role="status"
        >
          {{ t('plugins.catalog.marketplaceUnavailable') }}
        </p>
        <p
          v-if="tab === 'browse' && !exploring && searchError"
          class="plugin-catalog__error"
          role="alert"
        >
          {{ searchError }}
        </p>

        <slot
          v-if="tab === 'processors'"
          name="processors"
        />
        <!-- No `disabled` binding: it would be read before the right-clicked row is
             known. Rows without commands get an empty (hidden) menu. -->
        <BaseContextMenu
          v-else
          :items="menuItems"
          @open="onMenuOpen"
          @select="onMenu"
        >
          <ul
            v-if="rows.length"
            class="plugin-catalog__list"
            :aria-label="t('plugins.title')"
          >
            <li
              v-for="row in rows"
              :key="row.key"
              class="plugin-catalog__row"
              :class="{ 'plugin-catalog__row--off': row.kind === 'local' && row.plugin.installed && !row.plugin.enabled }"
              :data-plugin-key="row.key"
            >
              <BaseSwitch
                v-if="row.kind === 'local' && row.plugin.installed"
                class="plugin-catalog__control"
                :model-value="row.plugin.enabled"
                :aria-label="t('plugins.catalog.enable', { name: rowName(row) })"
                @update:model-value="emit('command', row.plugin.descriptor.id, 'toggle-enabled')"
              />
              <button
                class="plugin-catalog__open"
                type="button"
                @click="emit('open', row.key)"
              >
                <span class="plugin-catalog__name">{{ rowName(row) }}</span>
                <span class="plugin-catalog__sub">{{ rowSub(row) }}</span>
              </button>

              <template v-if="row.kind === 'local'">
                <span
                  v-if="tab === 'installed' && !row.plugin.available"
                  v-tooltip="row.plugin.unavailableReason ?? t('plugins.state.unavailable')"
                  class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
                >
                  <IconWarning />
                  {{ t('plugins.catalog.unavailable') }}
                </span>
                <template v-if="tab === 'installed'">
                  <span
                    v-tooltip="t('plugins.details.actions')"
                    class="plugin-catalog__meta plugin-catalog__control"
                    role="img"
                    :aria-label="`${t('plugins.details.actions')}: ${row.plugin.descriptor.actionTypeIds.length}`"
                  >
                    <IconPlay :size="14" />{{ row.plugin.descriptor.actionTypeIds.length }}
                  </span>
                  <span
                    v-tooltip="t('plugins.details.events')"
                    class="plugin-catalog__meta plugin-catalog__control"
                    role="img"
                    :aria-label="`${t('plugins.details.events')}: ${row.plugin.descriptor.eventTypeIds.length}`"
                  >
                    <IconZap :size="14" />{{ row.plugin.descriptor.eventTypeIds.length }}
                  </span>
                </template>
                <template v-else>
                  <span class="plugin-catalog__meta plugin-catalog__mono">{{ row.plugin.descriptor.version }}</span>
                  <BaseIconButton
                    class="plugin-catalog__control"
                    :label="t('plugins.catalog.installPlugin', { name: rowName(row) })"
                    variant="default"
                    size="sm"
                    :disabled="!row.plugin.available"
                    @click="emit('command', row.plugin.descriptor.id, 'install')"
                  >
                    <IconDownload />
                  </BaseIconButton>
                </template>
              </template>

              <template v-else-if="row.kind === 'remote'">
                <span class="plugin-catalog__meta plugin-catalog__mono">{{ row.entry.latest }}</span>
                <BaseIconButton
                  class="plugin-catalog__control"
                  :label="t('plugins.catalog.installPlugin', { name: rowName(row) })"
                  :hint="t('packages.remote.install')"
                  variant="default"
                  size="sm"
                  @click="emit('open', row.key)"
                >
                  <IconDownload />
                </BaseIconButton>
              </template>

              <template v-else>
                <span
                  v-if="row.update.currentStatus !== 'approved'"
                  class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
                >
                  {{ t(`packages.liveStatus.${row.update.currentStatus ?? 'unknown'}`) }}
                </span>
                <span
                  v-if="row.update.advisory"
                  class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
                >{{ t(`packages.advisory.${row.update.advisory.severity}`) }}</span>
                <span
                  v-if="row.update.latest && row.update.latestStatus !== 'approved'"
                  class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
                >
                  {{ t(`packages.liveStatus.${row.update.latestStatus ?? 'unknown'}`) }}
                </span>
                <span class="plugin-catalog__meta plugin-catalog__mono">
                  {{ row.update.latest ? `${row.update.current} → ${row.update.latest}` : row.update.current }}
                </span>
                <BaseIconButton
                  v-if="row.update.latest"
                  class="plugin-catalog__control"
                  :label="t('plugins.catalog.update', { name: rowName(row) })"
                  variant="default"
                  size="sm"
                  :disabled="checkingUpdates || row.update.latestStatus !== 'approved'"
                  @click="emit('update', row.update.id, row.update.latest)"
                >
                  <IconUpload />
                </BaseIconButton>
              </template>

              <IconChevronRight class="plugin-catalog__chevron" />
            </li>
          </ul>
          <BaseEmptyState
            v-else-if="tab === 'installed' && filtering"
            :title="t('plugins.catalog.noMatches', { query: query.trim() })"
          />
          <BaseEmptyState
            v-else-if="tab === 'installed'"
            :title="t('plugins.catalog.emptyInstalled')"
            :description="t('plugins.catalog.emptyInstalledHint')"
          >
            <template #icon>
              <IconPuzzle :size="28" />
            </template>
            <BaseButton
              size="sm"
              @click="tab = 'browse'"
            >
              {{ t('plugins.catalog.browseAvailable') }}
            </BaseButton>
          </BaseEmptyState>
          <BaseEmptyState
            v-else-if="tab === 'browse' && !exploring && marketplaceAvailable === false"
            :title="t('plugins.catalog.marketplaceUnavailable')"
            :description="t('plugins.catalog.marketplaceUnavailableHint')"
          >
            <BaseButton
              size="sm"
              @click="emit('install-package')"
            >
              {{ t('plugins.install.package') }}
            </BaseButton>
          </BaseEmptyState>
          <BaseEmptyState
            v-else-if="tab === 'browse' && !exploring"
            :title="searching ? t('plugins.catalog.searching') : t('plugins.catalog.emptyBrowse')"
            :description="t('plugins.catalog.emptyBrowseHint')"
          >
            <BaseButton
              size="sm"
              @click="emit('install-package')"
            >
              {{ t('plugins.install.package') }}
            </BaseButton>
          </BaseEmptyState>
          <BaseEmptyState
            v-else-if="tab === 'updates'"
            :title="filtering ? t('plugins.catalog.noMatches', { query: query.trim() }) : t('plugins.catalog.emptyUpdates')"
          />
          <ExploreList
            v-if="exploring"
            :installed-ids="installedIds"
            :hidden-ids="listedLocally"
            @open-plugin="openCatalogPlugin"
          />
        </BaseContextMenu>
      </div>
    </BaseTabs>
  </section>
</template>

<style scoped>
.plugin-catalog {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  color: var(--color-text);
}

.plugin-catalog__title {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.plugin-catalog__tabs {
  flex: 1;
}

.plugin-catalog__tabs :deep(.base-tabs__bar) {
  padding: var(--space-2) var(--space-3) 0;
  background: var(--color-bg-panel-header);
}

/* Search and the icon tools share the tab row; on narrow screens they wrap below it. */
.plugin-catalog__bar {
  display: flex;
  min-height: var(--control-height-md);
  flex: 1 1 240px;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
  padding-bottom: var(--space-2);
}

.plugin-catalog__search {
  flex: 1;
  min-width: 0;
  max-width: 320px;
  margin: 0;
}

.plugin-catalog__tools {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
}

.plugin-catalog__body {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: auto;
}

.plugin-catalog__error,
.plugin-catalog__notice {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}

.plugin-catalog__notice {
  color: var(--color-text-muted);
}

.plugin-catalog__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.plugin-catalog__row {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: calc(var(--control-height-lg) + var(--space-3));
  padding: 0 var(--space-4);
}

.plugin-catalog__row + .plugin-catalog__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-catalog__row:hover {
  background: var(--color-bg-hover);
}

/* The open button stretches over the whole row; switches, chips and buttons stay clickable above it. */
.plugin-catalog__open {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
  padding: var(--space-3) 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.plugin-catalog__open::after {
  position: absolute;
  inset: 0;
  content: '';
}

.plugin-catalog__open:focus-visible {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: calc(var(--border-width) * -1);
}

.plugin-catalog__control {
  position: relative;
  z-index: 1;
}

.plugin-catalog__name {
  overflow: hidden;
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-catalog__row--off .plugin-catalog__name {
  opacity: 0.6;
}

.plugin-catalog__sub {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-catalog__meta {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.plugin-catalog__mono {
  font-family: var(--font-family-mono);
}

.plugin-catalog__chip {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-3);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  line-height: 20px;
  white-space: nowrap;
}

.plugin-catalog__chip--warning {
  border: var(--border-width) solid var(--color-warning);
  color: var(--color-warning);
}

.plugin-catalog__chevron {
  flex: none;
  color: var(--color-text-muted);
}

@media (width < 600px) {
  .plugin-catalog__bar {
    flex-basis: 100%;
  }

  .plugin-catalog__search {
    max-width: none;
  }
}
</style>
