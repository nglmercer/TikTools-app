<script setup lang="ts">
import { computed, inject, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { routerKey } from 'vue-router';
import { isLocale } from '@/shared/i18n/supported-locales.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { startTour } from '../model/coordinator.ts';
import {
  tagIcons,
  tutorialEntries,
  tutorialSteps,
  tutorialTags,
  type TutorialTag,
} from '../model/tutorial-catalog.ts';
import type { TourCapabilities, TourId, TourLocale, TourTaskState } from '../model/types.ts';

const props = withDefaults(defineProps<{
  capabilities?: TourCapabilities;
  taskState?: TourTaskState | (() => TourTaskState);
  /** False when another modal must stay open (e.g. the library review): read steps only. */
  startable?: boolean;
  initialCategory?: TutorialTag;
  /** Restrict the catalog to a caller's offerings (the demo has no account/backup tour). */
  tours?: readonly TourId[];
  /** Preselect the entry recommended for the reader's current section. */
  initialTour?: TourId;
  /** Delegate launching, e.g. so the demo can prepare the matching scenario first. */
  onLaunch?: (id: TourId) => void;
}>(), { capabilities: undefined, taskState: undefined, startable: true, initialCategory: undefined, tours: undefined, initialTour: undefined, onLaunch: undefined });

type TabValue = 'all' | TutorialTag;

const open = defineModel<boolean>('open', { default: false });
const { t, locale } = useI18n();
const router = inject(routerKey, null);
const query = ref('');
const activeTab = ref<TabValue>(props.initialCategory ?? 'all');
const selectedId = ref<TourId | null>(props.initialTour ?? null);
const tourLocale = computed<TourLocale>(() => isLocale(locale.value) ? locale.value : 'en');

const text = (key: string): string => String(t(key));
const categoryOptions = computed(() => [
  { value: 'all' as TabValue, label: text('guides.catalog.allTags'), icon: IconRows },
  ...tutorialTags.map((tag) => ({
    value: tag as TabValue,
    label: text(`guides.catalog.tags.${tag}`),
    icon: tagIcons[tag],
  })),
]);
const allowedTours = computed(() => props.tours ? new Set<string>(props.tours) : null);
const tutorials = computed(() => tutorialEntries
  .filter((entry) => !allowedTours.value || allowedTours.value.has(entry.id))
  .map((entry) => ({
    ...entry,
    title: text(`guides.catalog.tours.${entry.id}.title`),
    description: text(`guides.catalog.tours.${entry.id}.description`),
    steps: tutorialSteps(tourLocale.value, entry.id),
  })));
const filtered = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase();
  return tutorials.value.filter((entry) => {
    if (activeTab.value !== 'all' && !entry.tags.includes(activeTab.value)) return false;
    if (!needle) return true;
    return [entry.title, entry.description, ...entry.tags.map((tag) => text(`guides.catalog.tags.${tag}`))]
      .join(' ').toLocaleLowerCase().includes(needle);
  });
});
const PAGE_SIZE = 6;
const page = ref(0);
/** Entries ordered by their primary tag following tutorialTags order, so groups stay contiguous across pages. */
const orderedEntries = computed(() => {
  const order = new Map(tutorialTags.map((tag, index) => [tag, index]));
  return [...filtered.value].sort((a, b) => (order.get(a.tags[0]!) ?? 99) - (order.get(b.tags[0]!) ?? 99));
});
const pageCount = computed(() => Math.max(1, Math.ceil(orderedEntries.value.length / PAGE_SIZE)));
const pagedEntries = computed(() => orderedEntries.value.slice(page.value * PAGE_SIZE, (page.value + 1) * PAGE_SIZE));
type ListItem =
  | { kind: 'header'; tag: TutorialTag }
  | { kind: 'entry'; entry: (typeof filtered.value)[number] };
const listItems = computed<readonly ListItem[]>(() => {
  const items: ListItem[] = [];
  let lastTag: TutorialTag | null = null;
  for (const entry of pagedEntries.value) {
    const tag = entry.tags[0]!;
    if (tag !== lastTag) { items.push({ kind: 'header', tag }); lastTag = tag; }
    items.push({ kind: 'entry', entry });
  }
  return items;
});
function prevPage(): void { if (page.value > 0) page.value -= 1; }
function nextPage(): void { if (page.value < pageCount.value - 1) page.value += 1; }
watch(filtered, () => { page.value = 0; });

const selected = computed(() => filtered.value.find((entry) => entry.id === selectedId.value) ?? filtered.value[0] ?? null);

watch(filtered, (list) => {
  if (!list.some((entry) => entry.id === selectedId.value)) selectedId.value = list[0]?.id ?? null;
}, { immediate: true });
watch(open, (isOpen) => {
  if (isOpen) {
    activeTab.value = props.initialCategory ?? 'all';
    selectedId.value = props.initialTour ?? null;
  } else { query.value = ''; activeTab.value = 'all'; selectedId.value = null; }
});

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
    taskState: () => (typeof props.taskState === 'function' ? props.taskState() : props.taskState) ?? {},
  });
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t('guides.catalog.title')"
    :width="760"
    class="tutorial-catalog"
  >
    <p class="tutorial-catalog__intro">
      {{ t(startable ? 'guides.catalog.description' : 'guides.catalog.readonlyHint') }}
    </p>
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
        v-model="activeTab"
        class="tutorial-catalog__category"
        :options="categoryOptions"
        :aria-label="t('guides.catalog.tagsLabel')"
      />
    </div>
    <div class="tutorial-catalog__layout">
      <TransitionGroup
        v-if="filtered.length"
        name="tutorial-catalog"
        tag="ul"
        class="tutorial-catalog__list"
      >
        <template
          v-for="item in listItems"
          :key="item.kind === 'header' ? `h-${item.tag}` : item.entry.id"
        >
          <li
            v-if="item.kind === 'header'"
            class="tutorial-catalog__group"
            aria-hidden="true"
          >
            <component
              :is="tagIcons[item.tag]"
              :size="13"
            />
            {{ t(`guides.catalog.tags.${item.tag}`) }}
          </li>
          <li v-else>
            <button
              type="button"
              class="tutorial-catalog__card"
              :class="{ 'tutorial-catalog__card--active': selected?.id === item.entry.id }"
              :aria-pressed="selected?.id === item.entry.id"
              @click="selectedId = item.entry.id"
            >
              <span class="tutorial-catalog__card-icon">
                <component
                  :is="item.entry.icon"
                  :size="18"
                />
              </span>
              <span class="tutorial-catalog__card-body">
                <strong>{{ item.entry.title }}</strong>
                <span class="tutorial-catalog__card-description">{{ item.entry.description }}</span>
                <span class="tutorial-catalog__card-meta">
                  <span class="tutorial-catalog__chip">
                    <component
                      :is="tagIcons[item.entry.tags[0]!]"
                      :size="12"
                    />
                    {{ t(`guides.catalog.tags.${item.entry.tags[0]}`) }}
                  </span>
                  <span>{{ t('guides.catalog.steps', { count: item.entry.steps.length }) }}</span>
                </span>
              </span>
            </button>
          </li>
        </template>
      </TransitionGroup>
      <div
        v-if="pageCount > 1"
        class="tutorial-catalog__pager"
      >
        <BaseButton
          variant="ghost"
          size="sm"
          :disabled="page === 0"
          :aria-label="t('guides.catalog.prevPage')"
          @click="prevPage"
        >
          ‹
        </BaseButton>
        <span class="tutorial-catalog__page-label">{{ t('guides.catalog.pageOf', { current: page + 1, total: pageCount }) }}</span>
        <BaseButton
          variant="ghost"
          size="sm"
          :disabled="page >= pageCount - 1"
          :aria-label="t('guides.catalog.nextPage')"
          @click="nextPage"
        >
          ›
        </BaseButton>
      </div>
      <p
        v-if="!filtered.length"
        class="tutorial-catalog__empty"
      >
        {{ t('guides.catalog.empty') }}
      </p>
      <aside
        v-if="selected"
        :key="selected.id"
        class="tutorial-catalog__detail"
      >
        <div class="tutorial-catalog__detail-head">
          <span class="tutorial-catalog__detail-icon">
            <component
              :is="selected.icon"
              :size="20"
            />
          </span>
          <div>
            <h3>{{ selected.title }}</h3>
            <p>{{ selected.description }}</p>
          </div>
        </div>
        <ol class="tutorial-catalog__step-list">
          <li
            v-for="(step, index) in selected.steps"
            :key="step.id"
          >
            <strong>{{ index + 1 }}. {{ step.title }}</strong>
            <span>{{ step.description }}</span>
          </li>
        </ol>
        <BaseButton
          v-if="startable"
          variant="primary"
          @click="launch(selected.id)"
        >
          {{ t('guides.catalog.start') }}
        </BaseButton>
      </aside>
    </div>
  </BaseDialog>
</template>

<style scoped>
.tutorial-catalog__intro { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.tutorial-catalog__filters { display: flex; gap: var(--space-2); margin-block: var(--space-3); }
.tutorial-catalog__search { display: flex; flex: 1; min-width: 0; }
.tutorial-catalog__category { flex: 0 0 210px; }
.tutorial-catalog__category :deep(.base-combo-box),
.tutorial-catalog__category :deep(.base-combo-box__trigger) { width: 100%; }
.tutorial-catalog__layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.05fr); gap: var(--space-4); padding-top: var(--space-2); }
.tutorial-catalog__list { display: grid; align-content: start; gap: var(--space-2); max-height: min(52vh, 480px); margin: 0; padding: 0 2px; overflow: auto; list-style: none; }
.tutorial-catalog__card {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  width: 100%;
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.tutorial-catalog__card:hover { border-color: var(--color-border-strong); }
.tutorial-catalog__card--active { border-color: var(--color-border-selected); background: var(--color-bg-control-active); }
.tutorial-catalog__card:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
.tutorial-catalog__card-icon,
.tutorial-catalog__detail-icon {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-accent);
}
.tutorial-catalog__detail-icon { width: 40px; height: 40px; }
.tutorial-catalog__card-body { display: grid; min-width: 0; gap: 2px; }
.tutorial-catalog__card-description { color: var(--color-text-muted); font-size: var(--font-size-sm); }
.tutorial-catalog__card-meta { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2); color: var(--color-text-subtle); font-size: var(--font-size-xs); }
.tutorial-catalog__chip { display: inline-flex; align-items: center; gap: var(--space-1); padding: 0 var(--space-2); border-radius: var(--radius-sm); background: var(--color-bg-control); color: var(--color-text-muted); }
.tutorial-catalog__detail { display: grid; align-content: start; gap: var(--space-4); min-width: 0; padding: var(--space-4); border: var(--border-width) solid var(--color-border-subtle); border-radius: var(--radius-md); background: var(--color-bg-panel-header); }
.tutorial-catalog__detail-head { display: flex; align-items: flex-start; gap: var(--space-3); }
.tutorial-catalog__detail-head h3 { margin: 0; font-size: var(--font-size-md); }
.tutorial-catalog__detail-head p { margin: 2px 0 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
.tutorial-catalog__step-list { display: grid; gap: var(--space-3); margin: 0; padding-inline-start: var(--space-5); font-size: var(--font-size-sm); }
.tutorial-catalog__step-list li { display: grid; gap: 2px; }
.tutorial-catalog__step-list span { color: var(--color-text-muted); }

.tutorial-catalog-enter-active,
.tutorial-catalog-leave-active,
.tutorial-catalog-move {
  transition:
    opacity var(--duration-normal) ease-out,
    transform var(--duration-normal) ease-out;
}

.tutorial-catalog-enter-from,
.tutorial-catalog-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

.tutorial-catalog__detail {
  animation: tutorial-detail-in var(--duration-normal) ease-out;
}

@keyframes tutorial-detail-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .tutorial-catalog-enter-active,
  .tutorial-catalog-leave-active,
  .tutorial-catalog-move,
  .tutorial-catalog__detail {
    transition-duration: 0.01ms;
    animation-duration: 0.01ms;
  }
}

.tutorial-catalog__group { display: flex; align-items: center; gap: var(--space-1); margin-top: var(--space-1); color: var(--color-text-subtle); font-size: var(--font-size-xs); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
.tutorial-catalog__pager { display: flex; align-items: center; justify-content: center; gap: var(--space-2); padding-top: var(--space-2); grid-column: 1; }
.tutorial-catalog__page-label { color: var(--color-text-muted); font-size: var(--font-size-sm); }
.tutorial-catalog__empty { margin: var(--space-4) 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }
@media (width < 760px) {
  .tutorial-catalog__filters { flex-wrap: wrap; }
  .tutorial-catalog__category { flex: 1 1 100%; }
  .tutorial-catalog__layout { grid-template-columns: minmax(0, 1fr); }
  .tutorial-catalog__list { max-height: 30vh; }
}
</style>
