<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, inject, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import { routeLocationKey, routerKey } from 'vue-router';
import type { ActionTypeDefinition, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import {
  availableActionTypeIds,
  createActionFromType,
  createEvent,
  isRecordSort,
  type RecordSort,
} from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import HotkeyIndicator from '@/entities/automation/ui/HotkeyIndicator.vue';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import type { AppliedRuleTemplate, RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import RuleTemplateDialog from '@/features/apply-rule-template/ui/RuleTemplateDialog.vue';
import ActionEditor from '@/features/edit-automation-action/ui/ActionEditor.vue';
import EventEditor from '@/features/edit-automation-event/ui/EventEditor.vue';
import ProfileImportDialog from '@/features/manage-rule-profiles/ui/ProfileImportDialog.vue';
import ProfileToolbar from '@/features/manage-rule-profiles/ui/ProfileToolbar.vue';
import ActionTypeDialog from '@/features/pick-action-type/ui/ActionTypeDialog.vue';
import { resolvedPacks, type RuleProfile } from '@/entities/rule-template/model/rule-profiles-state.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSplitter from '@/shared/ui/splitter/BaseSplitter.vue';
import { isLibraryTab, type LibraryTab } from '@/widgets/automation-library/model/library-tab.ts';
import AutomationLibrary from '@/widgets/automation-library/ui/AutomationLibrary.vue';
import AutomationRunsPanel from '@/widgets/automation-runs/ui/AutomationRunsPanel.vue';

type Editing =
  | { kind: 'event'; record: LiveEvent; isNew: boolean }
  | { kind: 'action'; record: LiveAction; isNew: boolean };

const { t } = useAutomationText();
const automation = useAutomationControls();
const { leaderboard } = storeToRefs(usePointsStore());
const saved = loadViewState().behavior;

const tab = ref<LibraryTab>(isLibraryTab(saved?.tab) ? saved.tab : 'events');
const selected = ref<string | null>(saved?.selected ?? null);
const eventQuery = ref(saved?.eventQuery ?? '');
const actionQuery = ref(saved?.actionQuery ?? '');
const eventSort = ref<RecordSort>(isRecordSort(saved?.eventSort) ? saved.eventSort : 'name');
const actionSort = ref<RecordSort>(isRecordSort(saved?.actionSort) ? saved.actionSort : 'name');
const draft = shallowRef<Editing | null>(null);
const dirty = ref(false);
const pending = shallowRef<(() => void) | null>(null);
const discardOpen = ref(false);
const pickerOpen = ref(false);
const templatesOpen = ref(false);
const profileImportOpen = ref(false);
const libraryWidth = ref(460);
const runsHeight = ref(200);
const isNarrow = useMediaQuery('(width < 900px)');

const snapshot = computed(() => automation.snapshot.value);
const eventTypes = computed(() => snapshot.value.eventTypes ?? []);
const packs = computed(() =>
  resolvedPacks(
    [...automation.profilePacks.value],
    snapshot.value.events.map((event) => event.id),
    snapshot.value.actions.map((action) => action.id),
  ),
);
const activePack = computed(() => packs.value.find((pack) => pack.id === automation.activeProfileId.value) ?? null);
const events = computed(() => {
  const pack = activePack.value;
  if (!pack) return snapshot.value.events;
  const ids = new Set(pack.eventIds);
  return snapshot.value.events.filter((event) => ids.has(event.id));
});
const actions = computed(() => {
  const pack = activePack.value;
  if (!pack) return snapshot.value.actions;
  const ids = new Set(pack.actionIds);
  return snapshot.value.actions.filter((action) => ids.has(action.id));
});
const availableTypes = computed(() => availableActionTypeIds(snapshot.value.plugins, snapshot.value.actionTypes));
const gifts = computed<SearchOption[]>(() =>
  automation.gifts.value.map((gift) => ({
    value: gift.name,
    label: gift.name,
    detail: `${gift.diamondCount} 💎`,
    iconUrl: gift.iconUrl,
  })),
);
const viewers = computed<SearchOption[]>(() =>
  leaderboard.value.map((viewer) => ({
    value: viewer.uniqueId,
    label: viewer.nickname ? `${viewer.nickname} (@${viewer.uniqueId})` : `@${viewer.uniqueId}`,
    detail: t('automations.pickers.points', { points: viewer.points }),
  })),
);

const editing = computed<Editing | null>(() => {
  if (draft.value) return draft.value;
  const id = selected.value;
  if (!id) return null;
  if (tab.value === 'events') {
    const record = snapshot.value.events.find((event) => event.id === id);
    return record ? { kind: 'event', record, isNew: false } : null;
  }
  const record = snapshot.value.actions.find((action) => action.id === id);
  return record ? { kind: 'action', record, isNew: false } : null;
});

const librarySelected = computed<string | null>({
  get: () => selected.value,
  set: (next) => guard(() => {
    draft.value = null;
    selected.value = next;
  }),
});
const libraryTab = computed<LibraryTab>({
  get: () => tab.value,
  set: (next) => guard(() => {
    draft.value = null;
    tab.value = next;
    selected.value = null;
  }),
});

function guard(run: () => void): void {
  if (!dirty.value) {
    run();
    return;
  }
  pending.value = run;
  discardOpen.value = true;
}

function confirmDiscard(): void {
  discardOpen.value = false;
  dirty.value = false;
  pending.value?.();
  pending.value = null;
}

function closeEditor(): void {
  guard(() => {
    draft.value = null;
    selected.value = null;
  });
}

function onSheetKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape' || event.defaultPrevented) return;
  if (!isNarrow.value || !editing.value) return;
  if (discardOpen.value || pickerOpen.value || templatesOpen.value || profileImportOpen.value) return;
  closeEditor();
}

function newEvent(trigger?: string): void {
  guard(() => {
    tab.value = 'events';
    const record = createEvent(t('automations.editor.newEventName'));
    if (trigger) record.trigger = trigger;
    selected.value = record.id;
    draft.value = { kind: 'event', record, isNew: true };
  });
}

// Other pages link here with `?trigger=<event type>` to start a rule for it.
// Optional injections: the page also renders without a router (tests, embeds).
const route = inject(routeLocationKey, null);
const router = inject(routerKey, null);
function openRequestedTrigger(): void {
  const trigger = route?.query.trigger;
  if (typeof trigger !== 'string' || !/^[a-z][a-z0-9_.-]{0,127}$/.test(trigger)) return;
  newEvent(trigger);
  void router?.replace({ query: {} });
}

function newAction(type: ActionTypeDefinition): void {
  guard(() => {
    tab.value = 'actions';
    const record = createActionFromType(type, localizedText);
    selected.value = record.id;
    draft.value = { kind: 'action', record, isNew: true };
  });
}

function saveEvent(event: LiveEvent): void {
  automation.saveEvent(event);
  dirty.value = false;
  draft.value = { kind: 'event', record: event, isNew: false };
}

function saveAction(action: LiveAction): void {
  automation.saveAction(action);
  dirty.value = false;
  draft.value = { kind: 'action', record: action, isNew: false };
}

function removeRecord(kind: LibraryTab, id: string): void {
  if (kind === 'events') automation.deleteEvent(id);
  else automation.deleteAction(id);
  dirty.value = false;
  draft.value = null;
  selected.value = null;
}

function setEnabled(kind: LibraryTab, id: string, enabled: boolean): void {
  if (kind === 'events') automation.setEventEnabled(id, enabled);
  else automation.setActionEnabled(id, enabled);
}

function importTemplates(templates: RuleTemplate[], done: (added: number) => void): void {
  void automation.importTemplates(templates).then(done, () => done(0));
}

function applyProfile(profile: RuleProfile, applied: AppliedRuleTemplate[]): void {
  void automation.applyProfile(profile, applied).then(() => {
    if (!automation.profileError.value) profileImportOpen.value = false;
  });
}

function switchProfile(id: string): void {
  guard(() => {
    draft.value = null;
    selected.value = null;
    void automation.switchProfile(id);
  });
}

watch(
  () => [snapshot.value.events, snapshot.value.actions] as const,
  () => {
    const current = draft.value;
    if (!current || current.isNew) return;
    const list: readonly { id: string }[] = current.kind === 'event' ? snapshot.value.events : snapshot.value.actions;
    if (list.some((record) => record.id === current.record.id) && !dirty.value) draft.value = null;
  },
);

const persist = debounce(() => {
  saveViewState('behavior', {
    tab: tab.value,
    selected: selected.value ?? undefined,
    eventQuery: eventQuery.value,
    eventSort: eventSort.value,
    actionQuery: actionQuery.value,
    actionSort: actionSort.value,
  });
});
watch([tab, selected, eventQuery, eventSort, actionQuery, actionSort], persist);

onMounted(() => {
  openRequestedTrigger();
  void automation.loadCustomTemplates();
  window.addEventListener('keydown', onSheetKeydown);
});

onBeforeUnmount(() => window.removeEventListener('keydown', onSheetKeydown));
</script>

<template>
  <div class="automations-page">
    <header class="automations-page__toolbar">
      <ProfileToolbar
        :packs="packs"
        :active-id="automation.activeProfileId.value"
        @switch="switchProfile"
        @create="automation.createProfile"
        @delete="automation.deleteProfile"
        @export="automation.exportProfile"
        @import="profileImportOpen = true"
      />
      <BaseButton
        size="sm"
        @click="templatesOpen = true"
      >
        {{ t('automations.templates.open') }}
      </BaseButton>
      <span class="automations-page__spacer" />
      <HotkeyIndicator
        :plugins="snapshot.plugins"
        :status="automation.hotkeyStatus.value"
        :last-event="automation.lastHotkeyEvent.value"
        :access-pending="automation.hotkeyAccessPending.value"
        @request-access="automation.requestHotkeyAccess()"
      />
    </header>
    <p
      v-if="automation.profileError.value"
      class="automations-page__banner automations-page__banner--error"
      role="alert"
    >
      {{ automation.profileError.value }}
    </p>
    <p
      v-else-if="automation.profileNotice.value"
      class="automations-page__banner"
      role="status"
    >
      {{ automation.profileNotice.value }}
    </p>

    <BaseSplitter
      v-if="!isNarrow"
      v-model:size="runsHeight"
      class="automations-page__workspace"
      orientation="vertical"
      :min="96"
      sized="end"
    >
      <template #start>
        <BaseSplitter
          v-model:size="libraryWidth"
          class="automations-page__main"
          :min="320"
          :max="760"
        >
          <template #start>
            <AutomationLibrary
              v-model:tab="libraryTab"
              v-model:selected="librarySelected"
              v-model:event-query="eventQuery"
              v-model:action-query="actionQuery"
              v-model:event-sort="eventSort"
              v-model:action-sort="actionSort"
              :events="events"
              :actions="actions"
              :action-types="snapshot.actionTypes"
              :available-types="availableTypes"
              :event-types="eventTypes"
              :runs="automation.runs.value"
              @new-event="newEvent"
              @new-action="pickerOpen = true"
              @set-enabled="setEnabled"
            />
          </template>
          <template #end>
            <EventEditor
              v-if="editing?.kind === 'event'"
              :key="editing.record.id"
              v-model:dirty="dirty"
              :event="editing.record"
              :is-new="editing.isNew"
              :actions="actions"
              :event-types="eventTypes"
              :gifts="gifts"
              :viewers="viewers"
              :test-runs="automation.testRuns.value"
              :hotkey-status="automation.hotkeyStatus.value"
              :error="automation.error.value"
              @save="saveEvent"
              @delete="removeRecord('events', $event)"
              @test="automation.testEvent"
              @fire="automation.fireEvent"
            />
            <ActionEditor
              v-else-if="editing?.kind === 'action'"
              :key="editing.record.id"
              v-model:dirty="dirty"
              :action="editing.record"
              :is-new="editing.isNew"
              :action-types="snapshot.actionTypes"
              :test-runs="automation.testRuns.value"
              :globals="automation.globals.value"
              :action-options="automation.actionOptions.value"
              :action-option-errors="automation.actionOptionErrors.value"
              :open-media-picker="automation.openMediaPicker"
              :error="automation.error.value"
              @save="saveAction"
              @delete="removeRecord('actions', $event)"
              @test="automation.testAction"
              @load-options="automation.loadOptions"
            />
            <div
              v-else
              class="automations-page__placeholder"
            >
              <BaseEmptyState
                :title="t('automations.editor.emptyTitle')"
                :description="t('automations.editor.emptyLead')"
              >
                <template #icon>
                  <IconFlow :size="28" />
                </template>
              </BaseEmptyState>
            </div>
          </template>
        </BaseSplitter>
      </template>
      <template #end>
        <AutomationRunsPanel :runs="automation.runs.value" />
      </template>
    </BaseSplitter>

    <template v-else>
      <div class="automations-page__narrow">
        <AutomationLibrary
          v-model:tab="libraryTab"
          v-model:selected="librarySelected"
          v-model:event-query="eventQuery"
          v-model:action-query="actionQuery"
          v-model:event-sort="eventSort"
          v-model:action-sort="actionSort"
          :events="events"
          :actions="actions"
          :action-types="snapshot.actionTypes"
          :available-types="availableTypes"
          :event-types="eventTypes"
          :runs="automation.runs.value"
          @new-event="newEvent"
          @new-action="pickerOpen = true"
          @set-enabled="setEnabled"
        />
        <AutomationRunsPanel
          class="automations-page__runs"
          :runs="automation.runs.value"
        />
      </div>
      <div
        v-if="editing"
        class="automations-page__sheet"
        role="dialog"
        aria-modal="true"
        :aria-label="editing.record.name"
      >
        <header class="automations-page__sheet-bar">
          <BaseButton
            size="sm"
            @click="closeEditor"
          >
            <template #icon>
              <IconChevronLeft />
            </template>
            {{ t('common.back') }}
          </BaseButton>
          <span class="automations-page__sheet-title">{{ editing.record.name }}</span>
        </header>
        <div class="automations-page__sheet-body">
          <EventEditor
            v-if="editing.kind === 'event'"
            :key="editing.record.id"
            v-model:dirty="dirty"
            :event="editing.record"
            :is-new="editing.isNew"
            :actions="actions"
            :event-types="eventTypes"
            :gifts="gifts"
            :viewers="viewers"
            :test-runs="automation.testRuns.value"
            :hotkey-status="automation.hotkeyStatus.value"
            :error="automation.error.value"
            @save="saveEvent"
            @delete="removeRecord('events', $event)"
            @test="automation.testEvent"
            @fire="automation.fireEvent"
          />
          <ActionEditor
            v-else
            :key="editing.record.id"
            v-model:dirty="dirty"
            :action="editing.record"
            :is-new="editing.isNew"
            :action-types="snapshot.actionTypes"
            :test-runs="automation.testRuns.value"
            :globals="automation.globals.value"
            :action-options="automation.actionOptions.value"
            :action-option-errors="automation.actionOptionErrors.value"
            :open-media-picker="automation.openMediaPicker"
            :error="automation.error.value"
            @save="saveAction"
            @delete="removeRecord('actions', $event)"
            @test="automation.testAction"
            @load-options="automation.loadOptions"
          />
        </div>
      </div>
    </template>

    <ActionTypeDialog
      v-model:open="pickerOpen"
      :plugins="snapshot.plugins"
      :action-types="snapshot.actionTypes"
      @pick="newAction"
      @open-plugins="automation.openPlugins()"
    />
    <RuleTemplateDialog
      v-model:open="templatesOpen"
      :snapshot="snapshot"
      :custom-templates="automation.customTemplates.value"
      :custom-error="automation.templateError.value"
      @apply="automation.applyTemplate"
      @import="importTemplates"
      @delete-custom="automation.deleteCustomTemplate"
    />
    <ProfileImportDialog
      v-model:open="profileImportOpen"
      :error="automation.profileError.value"
      @apply="applyProfile"
    />
    <BaseConfirmDialog
      v-model:open="discardOpen"
      :title="t('automations.editor.discardTitle')"
      :message="t('automations.editor.discardMessage', { name: editing?.record.name ?? '' })"
      :confirm-label="t('automations.editor.discardConfirm')"
      danger
      @confirm="confirmDiscard"
    />
  </div>
</template>

<style scoped>
.automations-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}

.automations-page__toolbar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
}

.automations-page__spacer {
  flex: 1;
}

.automations-page__banner {
  flex: none;
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-success);
  border-radius: var(--radius-sm);
  color: var(--color-success);
}

.automations-page__banner--error {
  border-color: var(--color-danger);
  color: var(--color-danger-hover);
}

.automations-page__workspace {
  flex: 1;
}

.automations-page__main {
  flex: 1;
}

.automations-page__placeholder {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

@media (width < 900px) {
  .automations-page {
    padding: var(--space-3);
    overflow-y: auto;
  }

  .automations-page__toolbar {
    flex-wrap: wrap;
    row-gap: var(--space-2);
  }

  .automations-page__narrow {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
  }

  .automations-page__narrow > :first-child {
    flex: none;
    height: min(64dvh, 560px);
  }

  .automations-page__runs {
    flex: none;
  }

  .automations-page__sheet {
    position: fixed;
    inset: 30px 0 0 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--color-bg-app);
  }

  .automations-page__sheet-bar {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-width) solid var(--color-border-subtle);
    background: var(--color-bg-panel-header);
  }

  .automations-page__sheet-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .automations-page__sheet-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
  }
}
</style>
