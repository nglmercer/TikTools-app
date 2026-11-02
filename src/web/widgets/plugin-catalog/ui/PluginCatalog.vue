<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import { catalogPlugins, pluginState, type PluginCatalogTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import PluginStateBadge from '@/entities/plugin/ui/PluginStateBadge.vue';
import type { SearchEntry, UpdateInfo } from '@/entities/plugin-package/api/package-types.ts';
import { remoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

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
}>();

const emit = defineEmits<{
  'install-package': [];
  'install-spec': [spec: string];
  search: [];
  'check-updates': [];
  update: [id: string, version?: string];
  'update-all': [];
  open: [id: string];
}>();

const tab = defineModel<PluginCatalogTab>('tab', { required: true });
const query = defineModel<string>('query', { default: '' });
const remoteQuery = defineModel<string>('remoteQuery', { default: '' });
const selected = defineModel<string | null>('selected', { default: null });

const { t } = useI18n();
const specInput = ref('');

const installedCount = computed(() => props.plugins.filter((plugin) => plugin.installed).length);
const pendingUpdates = computed(() => props.updates.filter((entry) => entry.latest));
const tabs = computed(() => [
  { value: 'installed' as const, label: `${t('plugins.catalog.installed')} (${installedCount.value})` },
  { value: 'browse' as const, label: t('plugins.catalog.browse') },
  {
    value: 'updates' as const,
    label: pendingUpdates.value.length
      ? `${t('plugins.catalog.updates')} (${pendingUpdates.value.length})`
      : t('plugins.catalog.updates'),
  },
]);

const rows = computed<CatalogRow[]>(() => {
  if (tab.value === 'updates') {
    return props.updates.map((update) => ({
      key: update.id,
      kind: 'update' as const,
      update,
      plugin: props.plugins.find((plugin) => plugin.descriptor.id === update.id),
    }));
  }
  const local = catalogPlugins(props.plugins, tab.value, props.actionTypes, query.value, localizedText).map(
    (plugin) => ({ key: plugin.descriptor.id, kind: 'local' as const, plugin }),
  );
  if (tab.value === 'installed') return local;
  const known = new Set(props.plugins.map((plugin) => plugin.descriptor.id));
  const remote = props.remote
    .filter((entry) => !known.has(entry.id))
    .map((entry) => ({ key: remoteSelection(entry.registry, entry.id), kind: 'remote' as const, entry }));
  return [...local, ...remote];
});

const filtering = computed(() => query.value.trim().length > 0);

function rowName(row: CatalogRow): string {
  if (row.kind === 'local') return localizedText(row.plugin.descriptor.name);
  if (row.kind === 'remote') return row.entry.name;
  return row.plugin ? localizedText(row.plugin.descriptor.name) : row.update.id;
}

function submitSpec(): void {
  const spec = specInput.value.trim();
  if (!spec) return;
  emit('install-spec', spec);
  specInput.value = '';
}
</script>

<template>
  <BasePanel
    class="plugin-catalog"
    :title="t('plugins.title')"
    flush
  >
    <div class="plugin-catalog__body">
      <div class="plugin-catalog__filters">
        <BaseSegmented
          v-model="tab"
          class="plugin-catalog__tabs"
          :options="tabs"
          size="sm"
        />
        <BaseTextField
          v-if="tab === 'installed'"
          v-model="query"
          type="search"
          size="sm"
          :placeholder="t('plugins.catalog.search')"
          :aria-label="t('plugins.catalog.search')"
        >
          <template #prefix>
            <IconSearch />
          </template>
        </BaseTextField>
        <template v-else-if="tab === 'browse'">
          <form
            class="plugin-catalog__row"
            @submit.prevent="emit('search')"
          >
            <BaseTextField
              v-model="remoteQuery"
              type="search"
              size="sm"
              :placeholder="t('plugins.catalog.searchRegistry')"
              :aria-label="t('plugins.catalog.searchRegistry')"
            >
              <template #prefix>
                <IconSearch />
              </template>
            </BaseTextField>
            <BaseButton
              size="sm"
              type="submit"
              :disabled="searching"
            >
              {{ t('plugins.catalog.searchButton') }}
            </BaseButton>
          </form>
          <form
            class="plugin-catalog__row"
            @submit.prevent="submitSpec"
          >
            <BaseTextField
              v-model="specInput"
              size="sm"
              :placeholder="t('plugins.catalog.installSpec')"
              :aria-label="t('plugins.catalog.installSpec')"
            />
            <BaseButton
              size="sm"
              type="submit"
              :disabled="!specInput.trim()"
            >
              {{ t('plugins.catalog.review') }}
            </BaseButton>
          </form>
          <p
            v-if="searchError"
            class="plugin-catalog__error"
            role="alert"
          >
            {{ searchError }}
          </p>
        </template>
        <div
          v-else
          class="plugin-catalog__row"
        >
          <BaseButton
            size="sm"
            :disabled="checkingUpdates"
            @click="emit('check-updates')"
          >
            {{ checkingUpdates ? t('plugins.catalog.checking') : t('plugins.catalog.checkUpdates') }}
          </BaseButton>
          <BaseButton
            size="sm"
            variant="primary"
            :disabled="!pendingUpdates.length"
            @click="emit('update-all')"
          >
            {{ t('plugins.catalog.updateAll') }}
          </BaseButton>
        </div>
      </div>
      <BaseList
        v-model:selected="selected"
        class="plugin-catalog__list"
        :items="rows"
        :item-key="(row) => row.key"
        :aria-label="t('plugins.title')"
        @activate="emit('open', $event.key)"
      >
        <template #default="{ item }">
          <span
            class="plugin-catalog__item"
            :class="{ 'plugin-catalog__item--off': item.kind === 'local' && item.plugin.installed && !item.plugin.enabled }"
            @pointerdown="emit('open', item.key)"
          >
            <IconPuzzle class="plugin-catalog__icon" />
            <span class="plugin-catalog__name">{{ rowName(item) }}</span>
            <PluginStateBadge
              v-if="item.kind === 'local' && (item.plugin.installed || !item.plugin.available)"
              :state="pluginState(item.plugin)"
            />
            <span
              v-else-if="item.kind === 'local'"
              class="plugin-catalog__tag"
            >{{ t('plugins.catalog.bundled') }}</span>
            <span
              v-else-if="item.kind === 'remote'"
              class="plugin-catalog__tag"
            >{{ item.entry.latest }}</span>
            <template v-else>
              <span
                v-if="item.update.advisory"
                class="plugin-catalog__tag plugin-catalog__tag--warning"
              >{{ t(`packages.advisory.${item.update.advisory.severity}`) }}</span>
              <BaseButton
                v-if="item.update.latest"
                size="sm"
                @click.stop="emit('update', item.update.id, item.update.latest)"
              >
                {{ item.update.current }} → {{ item.update.latest }}
              </BaseButton>
            </template>
          </span>
        </template>
        <template #empty>
          <BaseEmptyState
            v-if="tab === 'installed' && filtering"
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
            v-else-if="tab === 'browse'"
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
            v-else
            :title="t('plugins.catalog.emptyUpdates')"
          />
        </template>
      </BaseList>
    </div>
  </BasePanel>
</template>

<style scoped>
.plugin-catalog {
  height: 100%;
}

.plugin-catalog__body {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.plugin-catalog__filters {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.plugin-catalog__tabs {
  align-self: stretch;
}

.plugin-catalog__row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
}

.plugin-catalog__row > :first-child {
  flex: 1;
  min-width: 0;
}

.plugin-catalog__error {
  margin: 0;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}

.plugin-catalog__list {
  flex: 1;
  min-height: 0;
  border: 0;
  border-radius: 0;
}

.plugin-catalog__item {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.plugin-catalog__list :deep([aria-selected='true'] .plugin-state) {
  color: inherit;
}

.plugin-catalog__item--off .plugin-catalog__name {
  opacity: 0.6;
}

.plugin-catalog__icon {
  flex: none;
  opacity: 0.75;
}

.plugin-catalog__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-catalog__tag {
  flex: none;
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-catalog__tag--warning {
  color: var(--color-warning);
  font-family: inherit;
}
</style>
