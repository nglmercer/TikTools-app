<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CatalogItemSummary } from '@/entities/catalog/api/catalog-api.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { failureText } from '@/shared/api/cloud-failure.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';

import ExploreCatalogRow from './ExploreCatalogRow.vue';

/**
 * The Cloud catalog as rows, in the same list style as the installed plugins. A plugin opens
 * its details (and the signed package review installs it); a template or a profile is
 * imported through the single library import and its one owner review, with its automations
 * left disabled. An unavailable backend is a neutral state with a retry, never an outage of
 * the page; an empty plugin registry is "nothing published yet".
 */
const props = defineProps<{
  /** Plugin ids installed on this host. */
  installedIds: ReadonlySet<string>;
  pluginsOnly?: boolean;
  /** Plugin ids the page already lists itself (bundled, not installed yet); they are not repeated here. */
  hiddenIds?: ReadonlySet<string>;
}>();

const emit = defineEmits<{
  /** A plugin row was opened; `installed` plugins open their own details. */
  openPlugin: [id: string, installed: boolean];
}>();

const { t } = useI18n();
const catalog = useCatalogStore();
const account = useAccountStore();

const rows = computed(() =>
  catalog.items
    .filter((item) => (!props.pluginsOnly || item.kind === 'plugin') && (item.kind !== 'plugin' || !props.hiddenIds?.has(item.id)))
    .map((item) => ({ item, key: `${item.kind}:${item.id}` })),
);
const filtering = computed(() => catalog.filters.text.trim().length > 0);
const failed = computed(() => catalog.error !== null);
const signInNeeded = computed(() => {
  const code = catalog.errorDetail?.code;
  return code === 'unauthorized' || (catalog.filters.view === 'mine' && account.status?.state === 'signed-out');
});
/** An empty plugin registry is not an outage: templates and profiles are still served. */
const noPlugins = computed(() => catalog.dataState === 'noActiveGeneration' && catalog.filters.view !== 'mine');
const emptyTitle = computed(() => {
  if (filtering.value) return t('plugins.catalog.noMatches', { query: catalog.filters.text.trim() });
  return (props.pluginsOnly || noPlugins.value) ? t('exploreCatalog.nothingPublished') : t('exploreCatalog.noResults');
});
const emptyHint = computed(() => {
  if (props.pluginsOnly) return undefined;
  if (filtering.value) return undefined;
  return noPlugins.value ? t(props.pluginsOnly ? 'exploreCatalog.pluginsPublishedHint' : 'exploreCatalog.nothingPublishedHint') : t('exploreCatalog.noResultsHint');
});
const importNote = computed(() => {
  if (catalog.importState === 'staged') return { text: t('exploreCatalog.imported'), alert: false };
  if (catalog.importState === 'signIn') return { text: t('exploreCatalog.signInToImport'), alert: false };
  if (catalog.importState === 'failed') return { text: catalog.importMessage, alert: true };
  return null;
});

function isInstalled(item: CatalogItemSummary): boolean {
  return item.kind === 'plugin' && props.installedIds.has(item.id);
}

function openRow(item: CatalogItemSummary): void {
  if (item.kind === 'plugin') emit('openPlugin', item.id, isInstalled(item));
}

function retry(): void {
  void catalog.refresh();
}

/** False until the first request of this visit is answered, so the empty state never flashes. */
const settled = ref(false);
onMounted(async () => {
  if (props.pluginsOnly && (catalog.filters.type !== 'plugin' || catalog.filters.view !== 'all')) await catalog.setFilters({ ...catalog.filters, type: 'plugin', view: 'all' });
  else await catalog.refresh();
  settled.value = true;
});
</script>

<template>
  <div class="explore-list">
    <p
      v-if="importNote"
      class="explore-list__note"
      :class="{ 'explore-list__note--error': importNote.alert }"
      :role="importNote.alert ? 'alert' : 'status'"
    >
      {{ importNote.text }}
    </p>
    <p
      v-if="noPlugins && rows.length"
      class="explore-list__note"
      role="status"
    >
      {{ t('exploreCatalog.noPlugins') }}
    </p>

    <ul
      v-if="rows.length"
      class="explore-list__list"
      :aria-label="t('exploreCatalog.list')"
      :aria-busy="catalog.loading || undefined"
    >
      <ExploreCatalogRow
        v-for="{ item, key } in rows"
        :key="key"
        :item="item"
        :installed="isInstalled(item)"
        :importing="catalog.importState === 'working'"
        @open="openRow(item)"
        @install="catalog.install(item)"
        @import="catalog.importItem(item)"
      />
    </ul>

    <p
      v-else-if="catalog.loading || !settled"
      class="explore-list__note"
      role="status"
    >
      {{ t('unifiedCatalog.loading') }}
    </p>
    <BaseEmptyState
      v-else-if="failed"
      :title="t('exploreCatalog.unavailable')"
      :description="failureText(t, catalog.errorDetail, catalog.error ?? '') || t('exploreCatalog.unavailableHint')"
    >
      <template #icon>
        <IconCloud :size="28" />
      </template>
      <span class="explore-list__actions">
        <BaseIconButton
          :label="t('exploreCatalog.retry')"
          variant="default"
          size="sm"
          @click="retry"
        >
          <IconRefresh />
        </BaseIconButton>
        <BaseIconButton
          v-if="signInNeeded"
          :label="t('exploreCatalog.signIn')"
          variant="primary"
          size="sm"
          @click="account.startLogin()"
        >
          <IconUser />
        </BaseIconButton>
      </span>
    </BaseEmptyState>
    <BaseEmptyState
      v-else
      :title="emptyTitle"
      :description="emptyHint"
    >
      <template #icon>
        <IconCloud :size="28" />
      </template>
    </BaseEmptyState>

    <div
      v-if="catalog.hasMore && rows.length"
      class="explore-list__more"
    >
      <BaseIconButton
        :label="t('exploreCatalog.loadMore')"
        variant="default"
        size="sm"
        :disabled="catalog.loading"
        @click="catalog.loadMore()"
      >
        <IconChevronDown />
      </BaseIconButton>
    </div>
    <p
      v-if="failed && rows.length"
      class="explore-list__note explore-list__note--error"
      role="alert"
    >
      {{ failureText(t, catalog.errorDetail, catalog.error ?? '') }}
      <BaseIconButton
        :label="t('exploreCatalog.retry')"
        size="sm"
        @click="retry"
      >
        <IconRefresh />
      </BaseIconButton>
    </p>
  </div>
</template>

<style src="./explore-list.css"></style>
