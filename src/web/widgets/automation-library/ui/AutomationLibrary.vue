<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, watch } from 'vue';
import { recordSorts, type RecordSort } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconFilter from '@/shared/ui/icon/icons/IconFilter.vue';
import IconHash from '@/shared/ui/icon/icons/IconHash.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import type { LibraryArea, LibraryTab } from '../model/library-tab.ts';

import type { AutomationLibraryProps } from '../model/library-props.ts';
import { useAutomationLibraryRows } from '../model/use-automation-library-rows.ts';
import AutomationEventTable from './AutomationEventTable.vue';
import AutomationActionTable from './AutomationActionTable.vue';

const props = defineProps<AutomationLibraryProps>();

const emit = defineEmits<{
  newEvent: [];
  newAction: [];
  setEnabled: [kind: LibraryTab, id: string, enabled: boolean];
}>();

// The checkbox reflects committed state while the page confirms a transition.
// Remount only the last requested checkbox to undo its native optimistic tick.
const lastToggle = shallowRef<{ kind: LibraryTab; id: string; generation: number } | null>(null);
let toggleGeneration = 0;
function requestToggle(kind: LibraryTab, id: string, enabled: boolean): void {
  lastToggle.value = { kind, id, generation: ++toggleGeneration };
  emit('setEnabled', kind, id, enabled);
}
function checkboxKey(kind: LibraryTab, id: string): number {
  return lastToggle.value?.kind === kind && lastToggle.value.id === id ? lastToggle.value.generation : 0;
}

const tab = defineModel<LibraryArea>('tab', { required: true });
const selected = defineModel<string | null>('selected', { default: null });
const eventQuery = defineModel<string>('eventQuery', { default: '' });
const actionQuery = defineModel<string>('actionQuery', { default: '' });
const eventSort = defineModel<RecordSort>('eventSort', { default: 'name' });
const actionSort = defineModel<RecordSort>('actionSort', { default: 'name' });

const { t } = useAutomationText();

const tabs = computed(() => [
  { value: 'events' as const, label: `${t('automations.library.events')} (${props.events.length})`, icon: IconZap },
  { value: 'actions' as const, label: `${t('automations.library.actions')} (${props.actions.length})`, icon: IconPlay },
  ...(props.areas ?? []).map((area) => ({
    value: area,
    label: t(`automations.library.${area}`),
    icon: IconHash,
  })),
]);
// An extra area is built the first time it opens and then kept, so an unsaved draft survives switching tabs.
const visited = ref<ReadonlySet<LibraryArea>>(new Set([tab.value]));
watch(tab, (next) => { if (!visited.value.has(next)) visited.value = new Set([...visited.value, next]); });
const records = computed(() => tab.value === 'events' || tab.value === 'actions');
/** Sorting lives behind one icon to keep the toolbar minimal. */
const sortMenuOpen = ref(false);
const sortMenuAnchor = shallowRef<HTMLElement | null>(null);
const sortMenu = computed<MenuEntry[]>(() =>
  recordSorts.map((value) => ({ id: value, label: t(`automations.library.sorts.${value}`), checked: sort.value === value })),
);

function openSortMenu(event: MouseEvent): void {
  sortMenuAnchor.value = markRaw(event.currentTarget as HTMLElement);
  sortMenuOpen.value = true;
}

function onSortMenu(action: MenuAction): void {
  sort.value = action.id as RecordSort;
}

const { eventColumns, actionColumns, eventRows, actionRows } = useAutomationLibraryRows(props, {
  eventQuery, actionQuery, eventSort, actionSort,
});

const query = computed({
  get: () => (tab.value === 'events' ? eventQuery.value : actionQuery.value),
  set: (value: string) => {
    if (tab.value === 'events') eventQuery.value = value;
    else actionQuery.value = value;
  },
});
const sort = computed({
  get: () => (tab.value === 'events' ? eventSort.value : actionSort.value),
  set: (value: RecordSort | undefined) => {
    if (!value) return;
    if (tab.value === 'events') eventSort.value = value;
    else actionSort.value = value;
  },
});
</script>

<template>
  <section
    class="automation-library"
    :aria-label="t('automations.library.events')"
  >
    <BaseTabs
      v-model="tab"
      class="automation-library__tabs"
      :tabs="tabs"
    >
      <div
        v-if="records"
        class="automation-library__pane"
      >
        <div class="automation-library__toolbar">
          <BaseTextField
            v-model="query"
            class="automation-library__search"
            type="search"
            :placeholder="tab === 'events' ? t('automations.library.searchEvents') : t('automations.library.searchActions')"
            :aria-label="tab === 'events' ? t('automations.library.searchEvents') : t('automations.library.searchActions')"
          >
            <template #prefix>
              <IconSearch />
            </template>
          </BaseTextField>
          <slot
            name="sync"
            :kind="tab"
          />
          <BaseIconButton
            class="automation-library__sort"
            :class="{ 'automation-library__sort--active': sort !== 'name' }"
            :label="`${t('automations.library.sort')}: ${t(`automations.library.sorts.${sort}`)}`"
            variant="default"
            aria-haspopup="menu"
            :aria-expanded="sortMenuOpen"
            @click="openSortMenu"
          >
            <IconFilter />
          </BaseIconButton>
          <BaseMenu
            v-model:open="sortMenuOpen"
            :items="sortMenu"
            :anchor="sortMenuAnchor"
            placement="bottom-end"
            @select="onSortMenu"
          />
          <BaseButton
            variant="primary"
            @click="tab === 'events' ? emit('newEvent') : emit('newAction')"
          >
            <template #icon>
              <IconPlus />
            </template>
            {{ tab === 'events' ? t('automations.library.newEvent') : t('automations.library.newAction') }}
          </BaseButton>
        </div>
        <AutomationEventTable
          v-if="tab === 'events'"
          v-model:selected="selected"
          :columns="eventColumns"
          :rows="eventRows"
          :query="eventQuery"
          :checkbox-key="(id) => checkboxKey('events', id)"
          @set-enabled="(id, enabled) => requestToggle('events', id, enabled)"
        />
        <AutomationActionTable
          v-else
          v-model:selected="selected"
          :columns="actionColumns"
          :rows="actionRows"
          :query="actionQuery"
          :checkbox-key="(id) => checkboxKey('actions', id)"
          @set-enabled="(id, enabled) => requestToggle('actions', id, enabled)"
        />
      </div>
      <div
        v-if="areas?.includes('variables') && visited.has('variables')"
        v-show="tab === 'variables'"
        class="automation-library__pane automation-library__area"
      >
        <slot name="variables" />
      </div>
    </BaseTabs>
  </section>
</template>

<style src="./automation-library.css"></style>
