<script setup lang="ts">
import type { PluginCatalogProps } from '../model/catalog-types.ts';
import { usePluginCatalogRows } from '../model/use-plugin-catalog-rows.ts';
import PluginCatalogRow from './PluginCatalogRow.vue';

import { computed, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import { type PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import { isPluginCommand, pluginMenu, type PluginCommand } from '@/entities/plugin/lib/plugin-menu.ts';
import { OFFICIAL_REGISTRY, remoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import ExploreList from '@/features/explore-catalog/ui/ExploreList.vue';
import ExploreToolbar from '@/features/explore-catalog/ui/ExploreToolbar.vue';
import GuideButton from '@/features/guided-help/ui/GuideButton.vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconCompass from '@/shared/ui/icon/icons/IconCompass.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import BaseContextMenu from '@/shared/ui/menu/BaseContextMenu.vue';
import type { MenuAction } from '@/shared/ui/menu/menu-types.ts';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<PluginCatalogProps>();

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
const guideCapabilities = useCapabilityStore();
let installedBeforeRequest = new Set<string>();
const installRequested = ref(false);
function requestInstall(): void {
  installedBeforeRequest = new Set(props.plugins.filter((plugin) => plugin.installed).map((plugin) => plugin.descriptor.id));
  installRequested.value = true;
  emit('install-package');
}
const moreOpen = ref(false);
const moreAnchor = ref<HTMLElement | null>(null);
const moreItems = computed(() => [{ id: 'preferences', label: t('plugins.catalog.preferences'), icon: IconGear }]);
function toggleMore(event: MouseEvent): void { moreAnchor.value = event.currentTarget as HTMLElement; moreOpen.value = !moreOpen.value; }
const titleId = useId();
const { rows, installedCount, pendingUpdates, exploring, installedIds, listedLocally } = usePluginCatalogRows(props, tab, query);
/** Key of the row a context menu was opened on; never the open plugin, which has its own panel. */
const menuKey = ref<string | null>(null);

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
    data-guide-scope="plugins"
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
          <ExploreToolbar
            v-if="exploring"
            plugins-only
          />
          <form
            v-else-if="searchable"
            class="plugin-catalog__search"
            role="search"
            @submit.prevent="submitSearch"
          >
            <BaseTextField
              v-model="searchValue"
              data-guide="plugins-search"
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
            <BaseButton
              variant="primary"
              size="sm"
              data-guide="plugins-install"
              :aria-label="t('plugins.install.package')"
              :disabled="installDisabled"
              @click="requestInstall"
            >
              {{ t('plugins.install.button') }}
            </BaseButton>
            <GuideButton
              tour="plugin"
              :capabilities="guideCapabilities"
              :task-state="() => ({ 'plugin.installed': installRequested && plugins.some(plugin => plugin.installed && !installedBeforeRequest.has(plugin.descriptor.id)) })"
            />
            <BaseIconButton
              :label="t('syncChanges.more')"
              size="sm"
              aria-haspopup="menu"
              :aria-expanded="moreOpen"
              @click="toggleMore"
            ><IconMoreVertical /></BaseIconButton>
            <BaseMenu
              v-model:open="moreOpen"
              :items="moreItems"
              :anchor="moreAnchor"
              @select="emit('preferences')"
            />
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
            <PluginCatalogRow
              v-for="row in rows"
              :key="row.key"
              :row="row"
              :tab="tab"
              :checking-updates="checkingUpdates"
              @open="emit('open', $event)"
              @command="(id, command) => emit('command', id, command)"
              @update="(id, version) => emit('update', id, version)"
            />
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
            plugins-only
            :installed-ids="installedIds"
            :hidden-ids="listedLocally"
            @open-plugin="openCatalogPlugin"
          />
        </BaseContextMenu>
      </div>
    </BaseTabs>
  </section>
</template>

<style src="./plugin-catalog.css"></style>
