<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import type { CatalogTypeFilter } from '@/entities/catalog/model/catalog-query.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

/**
 * Search and filters of the Cloud catalog, as they sit in the tools row of Explore: a search
 * box, one toggle per kind (pressing the active kind shows everything again) and the owner's
 * own library. Text applies after a short pause; the filters apply at once.
 */
const props = defineProps<{ pluginsOnly?: boolean }>();
const SEARCH_DEBOUNCE_MS = 250;
const { t } = useI18n();
const catalog = useCatalogStore();
const draft = ref(catalog.filters.text);
let timer: ReturnType<typeof setTimeout> | undefined;

const kinds = computed(() => [
  { value: 'plugin' as const, label: t('exploreCatalog.kind.plugin'), icon: IconPuzzle },
  { value: 'template' as const, label: t('exploreCatalog.kind.template'), icon: IconFlow },
  { value: 'profile' as const, label: t('exploreCatalog.kind.profile'), icon: IconLayers },
]);

function applyText(): void {
  clearTimeout(timer);
  if (draft.value !== catalog.filters.text) void catalog.setFilters({ ...catalog.filters, text: draft.value, ...(props.pluginsOnly ? { type: 'plugin', view: 'all' } as const : {}) });
}

watch(draft, () => {
  clearTimeout(timer);
  timer = setTimeout(applyText, SEARCH_DEBOUNCE_MS);
});

function toggleKind(kind: Exclude<CatalogTypeFilter, 'all'>): void {
  clearTimeout(timer);
  void catalog.setFilters({ ...catalog.filters, text: draft.value, type: catalog.filters.type === kind ? 'all' : kind });
}

function toggleMine(): void {
  clearTimeout(timer);
  void catalog.setFilters({ ...catalog.filters, text: draft.value, view: catalog.filters.view === 'mine' ? 'all' : 'mine' });
}

onBeforeUnmount(() => clearTimeout(timer));
</script>

<template>
  <div class="explore-toolbar">
    <form
      class="explore-toolbar__search"
      role="search"
      @submit.prevent="applyText"
    >
      <BaseTextField
        v-model="draft"
        data-guide="plugins-search"
        type="search"
        size="sm"
        :placeholder="pluginsOnly ? t('plugins.catalog.search') : t('exploreCatalog.search')"
        :aria-label="pluginsOnly ? t('plugins.catalog.search') : t('exploreCatalog.search')"
      >
        <template #prefix>
          <IconSearch />
        </template>
      </BaseTextField>
    </form>
    <div
      v-if="!pluginsOnly"
      class="explore-toolbar__filters"
      role="group"
      :aria-label="t('exploreCatalog.kinds')"
    >
      <BaseIconButton
        v-for="kind in kinds"
        :key="kind.value"
        :label="kind.label"
        size="sm"
        :pressed="catalog.filters.type === kind.value"
        @click="toggleKind(kind.value)"
      >
        <component :is="kind.icon" />
      </BaseIconButton>
      <BaseIconButton
        :label="t('exploreCatalog.mine')"
        :hint="t('exploreCatalog.mineHint')"
        size="sm"
        :pressed="catalog.filters.view === 'mine'"
        @click="toggleMine"
      >
        <IconUser />
      </BaseIconButton>
    </div>
  </div>
</template>

<style scoped>
.explore-toolbar {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-3);
  min-width: 170px;
}

.explore-toolbar__search {
  flex: 1;
  min-width: 0;
  max-width: 320px;
  margin: 0;
}

.explore-toolbar__filters {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-1);
}

@media (width < 600px) {
  .explore-toolbar__search {
    max-width: none;
  }
}
</style>
