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
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

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
    .filter((item) => item.kind !== 'plugin' || !props.hiddenIds?.has(item.id))
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
  return noPlugins.value ? t('exploreCatalog.nothingPublished') : t('exploreCatalog.noResults');
});
const emptyHint = computed(() => {
  if (filtering.value) return undefined;
  return noPlugins.value ? t('exploreCatalog.nothingPublishedHint') : t('exploreCatalog.noResultsHint');
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

function kindLabel(item: CatalogItemSummary): string {
  return t(`unifiedCatalog.kinds.${item.kind}`);
}

function sub(item: CatalogItemSummary): string {
  return [item.publisher, item.summary].filter((part): part is string => Boolean(part)).join(' · ') || kindLabel(item);
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
  await catalog.refresh();
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
      <li
        v-for="{ item, key } in rows"
        :key="key"
        class="explore-list__row"
        :class="{ 'explore-list__row--off': item.yanked }"
        :data-catalog-key="key"
      >
        <span
          v-tooltip="kindLabel(item)"
          class="explore-list__kind"
          role="img"
          :aria-label="kindLabel(item)"
        >
          <IconPuzzle v-if="item.kind === 'plugin'" />
          <IconFlow v-else-if="item.kind === 'template'" />
          <IconLayers v-else />
        </span>
        <button
          v-if="item.kind === 'plugin'"
          class="explore-list__open"
          type="button"
          @click="openRow(item)"
        >
          <span class="explore-list__name">{{ item.title }}</span>
          <span class="explore-list__sub">{{ sub(item) }}</span>
        </button>
        <div
          v-else
          class="explore-list__open explore-list__open--static"
        >
          <span class="explore-list__name">{{ item.title }}</span>
          <span class="explore-list__sub">{{ sub(item) }}</span>
        </div>

        <span
          v-if="item.yanked"
          class="explore-list__chip explore-list__control"
        >
          <IconWarning />
          {{ t('unifiedCatalog.yanked') }}
        </span>
        <span
          v-if="isInstalled(item)"
          v-tooltip="t('exploreCatalog.installedBadge')"
          class="explore-list__meta explore-list__control"
          role="img"
          :aria-label="t('exploreCatalog.installedBadge')"
        >
          <IconCheck :size="14" />
        </span>
        <span
          v-if="item.version"
          class="explore-list__meta explore-list__mono"
        >{{ item.version }}</span>

        <BaseIconButton
          v-if="item.kind === 'plugin' && !isInstalled(item) && !item.yanked"
          class="explore-list__control"
          :label="t('exploreCatalog.install', { name: item.title })"
          :hint="t('packages.remote.install')"
          variant="default"
          size="sm"
          @click="catalog.install(item)"
        >
          <IconDownload />
        </BaseIconButton>
        <BaseIconButton
          v-else-if="item.kind !== 'plugin'"
          class="explore-list__control"
          :label="t(item.kind === 'template' ? 'exploreCatalog.importTemplate' : 'exploreCatalog.importProfile', { name: item.title })"
          variant="default"
          size="sm"
          :disabled="catalog.importState === 'working'"
          @click="catalog.importItem(item)"
        >
          <IconDownload />
        </BaseIconButton>
        <IconChevronRight
          v-if="item.kind === 'plugin'"
          class="explore-list__chevron"
        />
      </li>
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

<style scoped>
.explore-list {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.explore-list__note {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.explore-list__note--error {
  color: var(--color-danger-hover);
}

.explore-list__actions {
  display: inline-flex;
  gap: var(--space-2);
}

.explore-list__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.explore-list__row {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: calc(var(--control-height-lg) + var(--space-3));
  padding: 0 var(--space-4);
}

.explore-list__row + .explore-list__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.explore-list__row:hover {
  background: var(--color-bg-hover);
}

.explore-list__row--off .explore-list__name {
  opacity: 0.6;
}

.explore-list__kind {
  display: inline-flex;
  flex: none;
  color: var(--color-text-muted);
}

/* The open button stretches over the whole row; chips and buttons stay clickable above it. */
.explore-list__open {
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
}

button.explore-list__open {
  cursor: pointer;
}

button.explore-list__open::after {
  position: absolute;
  inset: 0;
  content: '';
}

button.explore-list__open:focus-visible {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: calc(var(--border-width) * -1);
}

.explore-list__control {
  position: relative;
  z-index: 1;
}

.explore-list__name {
  overflow: hidden;
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.explore-list__sub {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.explore-list__meta {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.explore-list__mono {
  font-family: var(--font-family-mono);
}

.explore-list__chip {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-warning);
  border-radius: var(--radius-sm);
  color: var(--color-warning);
  font-size: var(--font-size-sm);
  line-height: 20px;
  white-space: nowrap;
}

.explore-list__chevron {
  flex: none;
  color: var(--color-text-muted);
}

.explore-list__more {
  display: flex;
  justify-content: center;
  padding: var(--space-3);
  border-top: var(--border-width) solid var(--color-border-subtle);
}
</style>
