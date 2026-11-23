<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, markRaw, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import GuideCatalogButton from '@/features/guided-help/ui/GuideCatalogButton.vue';
import { eventFilters, filterEvents, windowEvents } from '@/entities/live-event/lib/event-filter.ts';
import { filterIcon } from '@/entities/live-event/lib/event-type-icon.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import EventRow from '@/entities/live-event/ui/EventRow.vue';
import LiveFeedStage from './LiveFeedStage.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconPause from '@/shared/ui/icon/icons/IconPause.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPopout from '@/shared/ui/icon/icons/IconPopout.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import type { DisplayEvent, EventFilter } from '@/types.ts';

const props = defineProps<{
  selectedEventId: number | null;
  inspectorOpen: boolean;
}>();

const emit = defineEmits<{
  select: [event: DisplayEvent];
  'open-viewer': [viewer: ViewerIdentity];
  'toggle-inspector': [];
}>();

const { t } = useI18n();
const store = useLiveFeedStore();
const connection = useConnectionStore();
const { events, filter, searchQuery, autoScroll, unreadCount, droppedUiEvents, renderLimit, imagesEnabled, liveViewers } = storeToRefs(store);
const { status } = storeToRefs(connection);
const panelTitle = computed(() => status.value === 'connected'
  ? t('feed.liveTitle', { count: liveViewers.value })
  : t('feed.title'));

type ViewMode = 'stage' | 'list';
const view = ref<ViewMode>('stage');
// The label names the destination, so a click is unambiguous without relying on the icon.
const viewLabel = computed(() => view.value === 'stage' ? t('feed.stage.switchToList') : t('feed.stage.switchToFeed'));
const viewIcon = computed(() => view.value === 'stage' ? IconRows : IconLayers);
function toggleView(): void { view.value = view.value === 'stage' ? 'list' : 'stage'; }

const filterOptions = computed(() => eventFilters.map((value) => ({ value, label: t(`feed.filters.${value}`), icon: filterIcon(value) })));
const activeFilter = computed<EventFilter>({ get: () => filter.value, set: (value) => store.setFilter(value) });
const query = computed({ get: () => searchQuery.value, set: (value: string) => store.setSearchQuery(value) });

const filtered = computed(() => filterEvents(events.value, filter.value, searchQuery.value));
const rendered = computed(() =>
  windowEvents(filtered.value, {
    autoScroll: autoScroll.value,
    filter: filter.value,
    query: searchQuery.value,
    limit: renderLimit.value,
  }),
);

// On narrow screens the secondary controls move into one overflow menu.
const isNarrow = useMediaQuery('(width < 700px)');
const guideButton = ref<InstanceType<typeof GuideCatalogButton> | null>(null);
const menuOpen = ref(false);
const menuAnchor = shallowRef<HTMLElement | null>(null);
const feedMenu = computed<MenuEntry[]>(() => [
  { id: 'guide', label: t('guides.catalog.button'), icon: IconInfo },
  { id: 'clear', label: t('feed.clear'), icon: IconTrash, disabled: events.value.length === 0 },
  { id: 'inspector', label: props.inspectorOpen ? t('feed.inspector.hide') : t('feed.inspector.show'), icon: IconPopout },
]);
function openMenu(event: MouseEvent): void {
  menuAnchor.value = markRaw(event.currentTarget as HTMLElement);
  menuOpen.value = true;
}
function onMenu(action: MenuAction): void {
  if (action.id === 'guide') guideButton.value?.open();
  else if (action.id === 'clear') store.resetEvents();
  else if (action.id === 'inspector') emit('toggle-inspector');
}
</script>

<template>
  <BasePanel
    class="live-feed"
    :title="panelTitle"
    flush
  >
    <template #actions>
      <BaseButton
        data-test="feed-view-toggle"
        data-guide="feed-view"
        size="sm"
        :aria-label="viewLabel"
        :title="viewLabel"
        @click="toggleView"
      >
        <template #icon>
          <component :is="viewIcon" />
        </template>
        {{ viewLabel }}
      </BaseButton>
      <BaseComboBox
        v-model="activeFilter"
        class="live-feed__filter"
        data-test="feed-filter-select"
        data-guide="feed-filters"
        :options="filterOptions"
        :aria-label="t('feed.filterLabel')"
      />
      <BaseTextField
        v-model="query"
        class="live-feed__search"
        type="search"
        size="sm"
        :placeholder="t('feed.search')"
        :aria-label="t('feed.search')"
      >
        <template #prefix>
          <IconSearch />
        </template>
      </BaseTextField>
      <div
        class="live-feed__controls"
        data-guide="feed-controls"
      >
        <GuideCatalogButton
          ref="guideButton"
          category="feed"
          :hide-button="isNarrow"
        />
        <BaseIconButton
          :label="autoScroll ? t('feed.pause') : t('feed.resume')"
          size="sm"
          :pressed="autoScroll"
          @click="store.handleToggleAutoScroll()"
        >
          <IconPause v-if="autoScroll" />
          <IconPlay v-else />
        </BaseIconButton>
        <BaseIconButton
          v-if="!isNarrow"
          :label="t('feed.clear')"
          size="sm"
          @click="store.resetEvents()"
        >
          <IconTrash />
        </BaseIconButton>
        <BaseIconButton
          v-if="!isNarrow"
          :label="inspectorOpen ? t('feed.inspector.hide') : t('feed.inspector.show')"
          size="sm"
          :pressed="inspectorOpen"
          @click="emit('toggle-inspector')"
        >
          <IconPopout />
        </BaseIconButton>
        <BaseIconButton
          v-if="isNarrow"
          :label="t('feed.moreActions')"
          size="sm"
          @click="openMenu"
        >
          <IconMoreVertical />
        </BaseIconButton>
      </div>
      <BaseMenu
        v-model:open="menuOpen"
        :items="feedMenu"
        :anchor="menuAnchor"
        @select="onMenu"
      />
    </template>

    <div
      :ref="(element) => store.setStreamContainerRef(element as Element | null)"
      class="live-feed__stream"
      data-guide="feed-stage"
      role="log"
      :aria-label="panelTitle"
    >
      <BaseEmptyState
        v-if="!filtered.length"
        :title="t('feed.empty')"
      >
        <template #icon>
          <IconChat :size="28" />
        </template>
        <div class="live-feed__intro-card">
          <p class="live-feed__intro-title">
            {{ t('feed.stage.guideCard') }}
          </p>
          <div class="live-feed__intro-tiles">
            <div class="live-feed__intro-tile">
              <span class="live-feed__intro-icon">
                <IconLayers :size="18" />
              </span>
              <strong>{{ t('feed.stage.persistentTitle') }}</strong>
              <p>{{ t('feed.stage.persistent') }}</p>
            </div>
            <div class="live-feed__intro-tile">
              <span class="live-feed__intro-icon">
                <IconZap :size="18" />
              </span>
              <strong>{{ t('feed.stage.ephemeralTitle') }}</strong>
              <p>{{ t('feed.stage.ephemeral') }}</p>
            </div>
          </div>
          <GuideCatalogButton
            category="feed"
            :label="t('feed.stage.openGuide')"
          />
        </div>
      </BaseEmptyState>
      <LiveFeedStage
        v-else-if="view === 'stage'"
        :events="rendered.visible"
        :selected-event-id="selectedEventId"
        :images="imagesEnabled"
        @select="emit('select', $event)"
        @open-viewer="emit('open-viewer', $event)"
      />
      <div
        v-else
        role="list"
      >
        <EventRow
          v-for="event in rendered.visible"
          :key="event.id"
          :event="event"
          :images="imagesEnabled"
          :selected="event.id === selectedEventId"
          @select="emit('select', $event)"
          @open-viewer="emit('open-viewer', $event)"
        />
      </div>
    </div>

    <div
      v-if="!autoScroll && unreadCount > 0"
      class="live-feed__jump"
    >
      <BaseButton
        variant="primary"
        size="sm"
        @click="store.handleToggleAutoScroll()"
      >
        <template #icon>
          <IconChevronDown />
        </template>
        {{ t('feed.jump', { count: unreadCount }) }}
      </BaseButton>
    </div>

    <template #footer>
      <span class="live-feed__count">{{ t('feed.count', { count: filtered.length }, filtered.length) }}</span>
      <span
        v-if="rendered.windowed"
        v-tooltip="t('feed.windowedHint')"
        class="live-feed__hint"
      >{{ t('feed.windowed', { count: rendered.visible.length }) }}</span>
      <span
        v-if="droppedUiEvents > 0"
        class="live-feed__hint live-feed__hint--warning"
      >{{ t('feed.dropped', { count: droppedUiEvents }) }}</span>
    </template>
  </BasePanel>
</template>

<style scoped>
.live-feed {
  position: relative;
}

:deep(.base-panel__header) {
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-3);
}

:deep(.base-panel__actions) {
  display: flex;
  flex: 1 1 auto;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  margin-left: auto;
}

.live-feed__search {
  flex: 1;
  min-width: 120px;
  max-width: 300px;
}

.live-feed__filter {
  flex: 0 0 132px;
}

.live-feed__filter :deep(.base-combo-box),
.live-feed__filter :deep(.base-combo-box__trigger) {
  width: 100%;
}

.live-feed__controls {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-1);
  min-width: 0;
}

.live-feed :deep(.base-empty-state) {
  min-height: 100%;
  justify-content: center;
}

.live-feed__intro-card {
  box-sizing: border-box;
  display: grid;
  gap: var(--space-4);
  width: min(100%, 560px);
  max-width: 100%;
  padding: var(--space-5);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  text-align: left;
}

.live-feed__intro-card .live-feed__intro-title {
  margin: 0;
  color: var(--color-text);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
  text-align: center;
}

.live-feed__intro-tiles {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}

.live-feed__intro-tile {
  display: grid;
  align-content: start;
  gap: var(--space-2);
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: var(--color-bg-panel-header);
}

.live-feed__intro-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-accent);
}

.live-feed__intro-tile strong {
  font-size: var(--font-size-sm);
}

.live-feed__intro-tile p {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}

.live-feed__intro-card > :last-child {
  justify-self: center;
}

.live-feed__stream {
  height: 100%;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.live-feed__jump {
  position: absolute;
  bottom: 44px;
  left: 50%;
  transform: translateX(-50%);
  box-shadow: var(--shadow-popup);
}

.live-feed__count {
  padding: 0 var(--space-3);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.live-feed__hint {
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.live-feed__hint--warning {
  color: var(--color-warning);
}

@media (width < 900px) {
  :deep(.base-panel__header) {
    flex-wrap: wrap;
  }

  :deep(.base-panel__actions) {
    flex: 1 1 100%;
    flex-wrap: wrap;
    margin-left: 0;
  }

  .live-feed__filter {
    flex: 0 0 112px;
  }

  .live-feed__search {
    flex: 1;
    min-width: 64px;
    max-width: none;
  }

  .live-feed__controls {
    gap: var(--space-1);
  }

  .live-feed__intro-tiles {
    grid-template-columns: minmax(0, 1fr);
  }

  .live-feed :deep(.base-empty-state) {
    padding: var(--space-3);
  }

  .live-feed__intro-card {
    padding: var(--space-3);
  }

  .live-feed__stream {
    min-height: 0;
  }
}
</style>
