<script setup lang="ts">
import { computed, inject, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { routerKey } from 'vue-router';
import { isLocale } from '@/shared/i18n/supported-locales.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { startTour } from '../model/coordinator.ts';
import { tourRegistry } from '../model/registry.ts';
import { automationActionExampleAvailable, automationActionExampleType, type TourRequirementSnapshot } from '../model/tour-requirements.ts';
import {
  categoryIcons,
  tagIcons,
  tutorialCard,
  tutorialCategories,
  tutorialEntries,
  tutorialSteps,
  tutorialTags,
  type LocalizedTutorialEntry,
  type TutorialCategory,
  type TutorialTag,
} from '../model/tutorial-catalog.ts';
import type { TourCapabilities, TourId, TourLocale, TourTaskState } from '../model/types.ts';
import TutorialCatalogHome from './tutorial-catalog/TutorialCatalogHome.vue';
import TutorialCategoryScreen from './tutorial-catalog/TutorialCategoryScreen.vue';
import TutorialDetailScreen from './tutorial-catalog/TutorialDetailScreen.vue';
import TutorialWizardNav from './tutorial-catalog/TutorialWizardNav.vue';
import type { TutorialCatalogScreen, TutorialCategorySummary } from './tutorial-catalog/types.ts';

const props = withDefaults(defineProps<{
  capabilities?: TourCapabilities;
  taskState?: TourTaskState | (() => TourTaskState);
  /** False when another modal must stay open (e.g. the library review): read steps only. */
  startable?: boolean;
  initialCategory?: TutorialCategory;
  /** Optional topic preselection inside the initial category (for automation tabs: events/actions/variables). */
  initialTag?: TutorialTag;
  /** Restrict the catalog to a caller's offerings (the demo has no account/backup tour). */
  tours?: readonly TourId[];
  /** Preselect the entry recommended for the reader's current section. */
  initialTour?: TourId;
  /** Delegate launching, e.g. so the demo can prepare the matching scenario first. */
  onLaunch?: (id: TourId) => void;
  /** Same-origin interactive preview URL supplied by hosts that bundle the demo. */
  previewUrl?: (id: TourId) => string;
  /** Explicit host account state. Undefined leaves availability filtering to the host's tour allow-list. */
  signedIn?: boolean;
  /** Automation body used to evaluate example-record requirements. Undefined is read-only/delegated catalog use, not proof that examples exist. */
  automationSnapshot?: TourRequirementSnapshot;
}>(), {
  capabilities: undefined,
  taskState: undefined,
  startable: true,
  initialCategory: undefined,
  initialTag: undefined,
  tours: undefined,
  initialTour: undefined,
  onLaunch: undefined,
  previewUrl: undefined,
  signedIn: undefined,
  automationSnapshot: undefined,
});

type CatalogFilter = 'all' | TutorialCategory | TutorialTag;

const open = defineModel<boolean>('open', { default: false });
const { t, locale } = useI18n();
const router = inject(routerKey, null);
const query = ref('');
const initialEntry = props.initialTour ? tutorialEntries.find((entry) => entry.id === props.initialTour) : undefined;
const initialCategory = props.initialCategory ?? initialEntry?.category ?? tutorialCategories[0];
const currentCategory = ref<TutorialCategory>(initialCategory);
const activeFilter = ref<CatalogFilter>(props.initialTag ?? 'all');
const selectedId = ref<TourId | null>(props.initialTour ?? null);
const screen = ref<TutorialCatalogScreen>(props.initialTour ? 'detail' : props.initialCategory ? 'category' : 'catalog');
const tourLocale = computed<TourLocale>(() => isLocale(locale.value) ? locale.value : 'en');

const text = (key: string): string => String(t(key));
const isCategory = (value: CatalogFilter): value is TutorialCategory => (tutorialCategories as readonly string[]).includes(value);
const isTag = (value: CatalogFilter): value is TutorialTag => (tutorialTags as readonly string[]).includes(value);
const allowedTours = computed(() => props.tours ? new Set<string>(props.tours) : null);
const accountAvailable = (id: TourId): boolean => props.signedIn === undefined || tourRegistry[id].availability?.account !== 'signed-in' || props.signedIn;
const requirementsMet = (id: TourId): boolean | undefined => {
  const requirement = tourRegistry[id].requirements?.automationActionExample;
  if (!requirement) return true;
  if (!props.automationSnapshot) return undefined;
  return automationActionExampleAvailable(requirement, props.automationSnapshot);
};
const tutorials = computed<LocalizedTutorialEntry[]>(() => tutorialEntries
  .filter((entry) => (!allowedTours.value || allowedTours.value.has(entry.id)) && accountAvailable(entry.id))
  .map((entry) => ({
    ...entry,
    title: tutorialCard(tourLocale.value, entry.id).title,
    description: tutorialCard(tourLocale.value, entry.id).description,
    steps: tutorialSteps(tourLocale.value, entry.id),
    requirementsMet: requirementsMet(entry.id),
  })));

const visibleCategories = computed(() => tutorialCategories
  .filter((category) => tutorials.value.some((entry) => entry.category === category)));
const categories = computed<readonly TutorialCategorySummary[]>(() => visibleCategories.value.map((id) => {
  const entries = tutorials.value.filter((entry) => entry.category === id);
  return {
    id,
    label: text(`guides.catalog.tags.${id}`),
    icon: categoryIcons[id],
    count: entries.length,
    entries,
  };
}));
const category = computed(() => categories.value.find((item) => item.id === currentCategory.value) ?? categories.value[0] ?? null);
const categoryTags = computed<readonly TutorialTag[]>(() => {
  if (!category.value) return [];
  const present = new Set(category.value.entries.flatMap((entry) => [...entry.tags]));
  return tutorialTags.filter((tag) => present.has(tag));
});
const selectOptions = computed(() => {
  const all = { value: 'all' as CatalogFilter, label: text('guides.catalog.allTags'), icon: IconRows };
  if (screen.value === 'catalog') {
    return [all, ...visibleCategories.value.map((id) => ({
      value: id as CatalogFilter,
      label: text(`guides.catalog.tags.${id}`),
      icon: categoryIcons[id],
    }))];
  }
  return [all, ...categoryTags.value.map((tag) => ({
    value: tag as CatalogFilter,
    label: text(`guides.catalog.tags.${tag}`),
    icon: tagIcons[tag],
  }))];
});
const showFilterSelect = computed(() => screen.value === 'catalog' || categoryTags.value.length > 1);
const activeTag = computed<'all' | TutorialTag>(() => activeFilter.value !== 'all' && isTag(activeFilter.value) ? activeFilter.value : 'all');
const visibleCategoryCount = computed(() => {
  const entries = category.value?.entries ?? [];
  return activeTag.value === 'all' ? entries.length : entries.filter((entry) => entry.tags.includes(activeTag.value as TutorialTag)).length;
});

const searching = computed(() => query.value.trim().length > 0);
const searchResults = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase();
  if (!needle) return [];
  return tutorials.value.filter((entry) => [
    entry.title,
    entry.description,
    text(`guides.catalog.tags.${entry.category}`),
    ...entry.tags.map((tag) => text(`guides.catalog.tags.${tag}`)),
  ].join(' ').toLocaleLowerCase().includes(needle));
});

const selected = computed(() => tutorials.value.find((entry) => entry.id === selectedId.value) ?? null);

function focusDetail(): void {
  void nextTick().then(() => {
    document.querySelector<HTMLElement>('dialog[open] .tutorial-catalog__detail')?.focus({ preventScroll: true });
  });
}
function focusTutorialCard(id: TourId): void {
  void nextTick().then(() => {
    document.querySelector<HTMLElement>(`dialog[open] [data-tutorial-id="${id}"]`)?.focus({ preventScroll: true });
  });
}

function tagForCategory(id: TutorialCategory, preferred: 'all' | TutorialTag = 'all'): 'all' | TutorialTag {
  if (preferred === 'all') return 'all';
  const entries = tutorials.value.filter((entry) => entry.category === id);
  return entries.some((entry) => entry.tags.includes(preferred)) ? preferred : 'all';
}
function openCategory(id: TutorialCategory, preferredTag: 'all' | TutorialTag = 'all'): void {
  query.value = '';
  selectedId.value = null;
  currentCategory.value = id;
  activeFilter.value = tagForCategory(id, preferredTag);
  screen.value = 'category';
}
function chooseFilter(value: CatalogFilter | undefined): void {
  if (!value) return;
  query.value = '';
  if (screen.value === 'catalog') {
    if (value !== 'all' && isCategory(value)) openCategory(value);
    return;
  }
  if (value !== 'all' && !isTag(value)) return;
  activeFilter.value = value;
  if (screen.value === 'detail') {
    selectedId.value = null;
    screen.value = 'category';
  }
}
function openTutorial(id: TourId): void {
  const entry = tutorials.value.find((item) => item.id === id);
  if (!entry) return;
  currentCategory.value = entry.category;
  if (activeFilter.value !== 'all' && (!isTag(activeFilter.value) || !entry.tags.includes(activeFilter.value))) activeFilter.value = 'all';
  query.value = '';
  selectedId.value = id;
  screen.value = 'detail';
  focusDetail();
}
function navigateWizard(target: 'catalog' | 'category'): void {
  const focusId = target === 'category' ? selectedId.value : null;
  selectedId.value = null;
  screen.value = target === 'category' && category.value ? 'category' : 'catalog';
  if (screen.value === 'catalog') activeFilter.value = 'all';
  if (focusId) focusTutorialCard(focusId);
}
function backToCategory(): void {
  const focusId = selectedId.value;
  selectedId.value = null;
  screen.value = category.value ? 'category' : 'catalog';
  if (focusId) focusTutorialCard(focusId);
}

watch(visibleCategories, (visible) => {
  if (!(visible as readonly TutorialCategory[]).includes(currentCategory.value)) currentCategory.value = visible[0] ?? tutorialCategories[0];
}, { immediate: true });
watch(query, (value) => {
  if (value.trim()) {
    activeFilter.value = 'all';
    screen.value = 'catalog';
  }
});
watch(open, (isOpen) => {
  if (isOpen) {
    query.value = '';
    selectedId.value = props.initialTour ?? null;
    const entry = props.initialTour ? tutorialEntries.find((item) => item.id === props.initialTour) : undefined;
    const selectedCategory = props.initialCategory ?? entry?.category;
    currentCategory.value = selectedCategory ?? visibleCategories.value[0] ?? tutorialCategories[0];
    activeFilter.value = selectedCategory ? tagForCategory(selectedCategory, props.initialTag ?? 'all') : 'all';
    screen.value = props.initialTour ? 'detail' : props.initialCategory ? 'category' : 'catalog';
  } else {
    query.value = '';
    activeFilter.value = 'all';
    selectedId.value = null;
    screen.value = 'catalog';
  }
});

function prepareRequirement(id: TourId): void {
  const requirement = tourRegistry[id].requirements?.automationActionExample;
  const type = requirement && props.automationSnapshot ? automationActionExampleType(requirement, props.automationSnapshot) : undefined;
  const createAction = type ? localizedText(type.title) : requirement?.typeId ?? requirement?.tag;
  if (!createAction || !router) return;
  open.value = false;
  void router.push({ path: '/automations', query: { createAction } });
}

async function launch(id: TourId): Promise<void> {
  open.value = false;
  // The coordinator refuses to run while a modal dialog is open, so let
  // BaseDialog close (post-flush watch) before starting the driver.js session.
  await nextTick();
  if (props.onLaunch) { props.onLaunch(id); return; }
  startTour(id, {
    router: router ?? undefined,
    locale: tourLocale.value,
    capabilities: props.capabilities,
    signedIn: props.signedIn,
    automationSnapshot: () => props.automationSnapshot,
    taskState: () => (typeof props.taskState === 'function' ? props.taskState() : props.taskState) ?? {},
  });
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t('guides.catalog.title')"
    :width="980"
    class="tutorial-catalog"
  >
    <div class="tutorial-catalog__masthead">
      <TutorialWizardNav
        :screen="screen"
        :category-label="screen === 'catalog' ? undefined : category?.label"
        :category-icon="screen === 'catalog' ? undefined : category?.icon"
        :category-count="screen === 'catalog' ? undefined : visibleCategoryCount"
        @navigate="navigateWizard"
      />
      <div class="tutorial-catalog__filters">
        <BaseTextField
          v-model="query"
          class="tutorial-catalog__search"
          type="search"
          :placeholder="t('guides.catalog.search')"
          :aria-label="t('guides.catalog.search')"
        >
          <template #prefix>
            <IconSearch />
          </template>
        </BaseTextField>
        <BaseComboBox
          v-if="showFilterSelect"
          v-model="activeFilter"
          class="tutorial-catalog__category-select"
          :options="selectOptions"
          :aria-label="t('guides.catalog.tagsLabel')"
          @update:model-value="chooseFilter"
        />
      </div>
    </div>

    <TutorialCatalogHome
      v-if="screen === 'catalog'"
      :categories="categories"
      :searching="searching"
      :results="searchResults"
      @open-category="openCategory"
      @open-tutorial="openTutorial"
    />
    <TutorialCategoryScreen
      v-else-if="screen === 'category' && category"
      :category="category"
      :active-tag="activeTag"
      @open-tutorial="openTutorial"
    />
    <TutorialDetailScreen
      v-else-if="screen === 'detail' && selected"
      :key="selected.id"
      :entry="selected"
      :startable="startable"
      :preview-url="props.previewUrl"
      :category-label="category?.label"
      @back="backToCategory"
      @start="launch"
      @prepare-requirement="prepareRequirement"
    />
  </BaseDialog>
</template>

<style src="./tutorial-catalog/tutorial-catalog.css"></style>
