<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import type { CatalogItemSummary } from '@/entities/catalog/api/catalog-api.ts';
import type { CatalogFilters } from '@/entities/catalog/model/catalog-query.ts';
import CatalogToolbar from '@/widgets/catalog-toolbar/ui/CatalogToolbar.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import { MAX_INSTALLED_LOOKUPS } from '@/entities/catalog/model/catalog-store.ts';

const MAX_INSTALLED = MAX_INSTALLED_LOOKUPS;

const props = defineProps<{
  /** Plugin identifiers already installed on this host. */
  installedIds: ReadonlySet<string>;
  /** Host target used for compatibility filtering. */
  target?: string;
}>();
const emit = defineEmits<{ manageProcessors: [] }>();

const { t } = useI18n();
const catalog = useCatalogStore();
const account = useAccountStore();

const SEARCH_DEBOUNCE_MS = 250;
const draft = ref<CatalogFilters>({ ...catalog.filters });
const signedIn = computed(() => account.status?.state === 'signed-in');
const selectedKey = computed(() => (catalog.detail ? `${catalog.detail.kind}:${catalog.detail.id}` : null));
let searchTimer: ReturnType<typeof setTimeout> | undefined;

function kindLabel(kind: CatalogItemSummary['kind']): string {
  return t(`unifiedCatalog.kinds.${kind}`);
}

function canInstall(item: CatalogItemSummary): boolean {
  return item.kind === 'plugin' && !item.yanked && !props.installedIds.has(item.id);
}

/** Text is applied after a short pause; other filters apply at once. */
function onFilters(next: CatalogFilters): void {
  draft.value = next;
  clearTimeout(searchTimer);
  if (next.text !== catalog.filters.text) {
    searchTimer = setTimeout(() => void catalog.setFilters(next), SEARCH_DEBOUNCE_MS);
  } else {
    void catalog.setFilters(next);
  }
}

function onAction(id: 'manageProcessors'): void {
  if (id === 'manageProcessors') emit('manageProcessors');
}

watch(() => props.installedIds, (ids) => catalog.setInstalled(ids), { immediate: true });
// A host target change reloads the first page; mount loads the initial one.
watch(() => props.target, (target) => void catalog.setTarget(target));

onMounted(() => {
  void catalog.setTarget(props.target);
});
onBeforeUnmount(() => clearTimeout(searchTimer));
</script>

<template>
  <section class="unified-catalog">
    <CatalogToolbar
      :model-value="draft"
      :signed-in="signedIn"
      @update:model-value="onFilters"
      @action="onAction"
      @sign-in="account.startLogin()"
    />

    <BaseAlert
      v-if="catalog.error"
      tone="error"
      :title="catalog.error"
      compact
    />

    <p
      v-if="catalog.installedLimited"
      class="unified-catalog__note"
    >
      {{ t('unifiedCatalog.installedLimited', { max: MAX_INSTALLED }) }}
    </p>

    <BaseEmptyState
      v-if="!catalog.loading && catalog.items.length === 0"
      :title="t('unifiedCatalog.empty')"
      :description="t('unifiedCatalog.emptyHint')"
    />

    <ul
      v-else
      class="unified-catalog__list"
    >
      <li
        v-for="item in catalog.items"
        :key="`${item.kind}:${item.id}`"
      >
        <button
          type="button"
          class="unified-catalog__row"
          :aria-pressed="selectedKey === `${item.kind}:${item.id}`"
          @click="catalog.openDetail(item)"
        >
          <span class="unified-catalog__title">{{ item.title }}</span>
          <span class="unified-catalog__meta">
            <span>{{ kindLabel(item.kind) }}</span>
            <span v-if="item.version">{{ t('unifiedCatalog.version', { version: item.version }) }}</span>
            <span v-if="item.publisher">{{ t('unifiedCatalog.publisher', { publisher: item.publisher }) }}</span>
            <span v-if="item.yanked">{{ t('unifiedCatalog.yanked') }}</span>
            <span v-if="!item.published">{{ t('unifiedCatalog.unpublished') }}</span>
            <span v-else-if="installedIds.has(item.id)">{{ t('unifiedCatalog.installed') }}</span>
          </span>
          <span
            v-if="item.summary"
            class="unified-catalog__summary"
          >{{ item.summary }}</span>
        </button>
      </li>
    </ul>

    <div
      v-if="catalog.hasMore"
      class="unified-catalog__more"
    >
      <BaseButton
        :disabled="catalog.loading"
        @click="catalog.loadMore()"
      >
        {{ catalog.loading ? t('unifiedCatalog.loading') : t('unifiedCatalog.loadMore') }}
      </BaseButton>
    </div>

    <aside
      v-if="catalog.detail"
      class="unified-catalog__detail"
      :aria-label="catalog.detail.title"
    >
      <header>
        <h3>{{ catalog.detail.title }}</h3>
        <BaseButton
          variant="ghost"
          size="sm"
          @click="catalog.closeDetail()"
        >
          {{ t('unifiedCatalog.close') }}
        </BaseButton>
      </header>
      <p>{{ catalog.detail.summary }}</p>
      <dl>
        <dt>{{ t('unifiedCatalog.type') }}</dt>
        <dd>{{ kindLabel(catalog.detail.kind) }}</dd>
        <template v-if="catalog.detail.version">
          <dt>{{ t('unifiedCatalog.latest') }}</dt>
          <dd>{{ catalog.detail.version }}</dd>
        </template>
        <template v-if="catalog.detail.publisher">
          <dt>{{ t('unifiedCatalog.publisherLabel') }}</dt>
          <dd>{{ catalog.detail.publisher }}</dd>
        </template>
        <template v-if="catalog.detail.targets.length">
          <dt>{{ t('unifiedCatalog.targets') }}</dt>
          <dd>{{ catalog.detail.targets.join(', ') }}</dd>
        </template>
      </dl>
      <BaseButton
        v-if="canInstall(catalog.detail)"
        variant="primary"
        @click="catalog.install(catalog.detail)"
      >
        {{ t('unifiedCatalog.install') }}
      </BaseButton>
      <template v-else-if="catalog.detail.kind === 'template' && catalog.detail.published">
        <BaseButton
          variant="primary"
          :disabled="catalog.importState === 'working'"
          @click="catalog.importTemplate(catalog.detail)"
        >
          {{ t('unifiedCatalog.importTemplate') }}
        </BaseButton>
        <p
          v-if="catalog.importState === 'staged'"
          class="unified-catalog__note"
        >
          {{ t('unifiedCatalog.templateStaged') }}
        </p>
        <p
          v-else-if="catalog.importState === 'signIn'"
          class="unified-catalog__note"
        >
          {{ t('unifiedCatalog.signInToImport') }}
        </p>
        <p
          v-else-if="catalog.importState === 'invalid'"
          class="unified-catalog__note"
        >
          {{ t('unifiedCatalog.templateInvalid') }}
        </p>
        <BaseAlert
          v-else-if="catalog.importState === 'failed'"
          tone="error"
          :title="catalog.importMessage"
          compact
        />
      </template>
      <p
        v-else-if="catalog.detail.kind === 'profile'"
        class="unified-catalog__note"
      >
        {{ t('unifiedCatalog.profileLater') }}
      </p>
      <p
        v-else-if="!catalog.detail.published"
        class="unified-catalog__note"
      >
        {{ t('unifiedCatalog.localOnly') }}
      </p>
    </aside>
  </section>
</template>

<style scoped>
.unified-catalog { display:grid; gap:var(--space-3); min-width:0; }
.unified-catalog__list { list-style:none; margin:0; padding:0; display:grid; gap:var(--space-2); }
.unified-catalog__row { width:100%; display:grid; gap:var(--space-1); text-align:start; padding:var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); background:var(--color-surface); cursor:pointer; min-width:0; }
.unified-catalog__row[aria-pressed="true"] { border-color:var(--color-accent); }
.unified-catalog__row:focus-visible { outline:2px solid var(--color-focus); outline-offset:2px; }
.unified-catalog__title { font-weight:600; overflow-wrap:anywhere; }
.unified-catalog__meta { display:flex; flex-wrap:wrap; gap:var(--space-2); font-size:var(--font-size-sm); color:var(--color-text-muted); }
.unified-catalog__summary { overflow-wrap:anywhere; }
.unified-catalog__more { display:flex; justify-content:center; }
.unified-catalog__detail { display:grid; gap:var(--space-2); padding:var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); min-width:0; }
.unified-catalog__detail header { display:flex; justify-content:space-between; gap:var(--space-2); align-items:start; }
.unified-catalog__detail h3 { margin:0; overflow-wrap:anywhere; }
.unified-catalog__detail dl { display:grid; grid-template-columns:auto 1fr; gap:var(--space-1) var(--space-3); margin:0; }
.unified-catalog__detail dt { color:var(--color-text-muted); }
.unified-catalog__detail dd { margin:0; overflow-wrap:anywhere; }
.unified-catalog__note { color:var(--color-text-muted); margin:0; }
</style>
