<script setup lang="ts">
import { useAutomationLibraryData } from '../model/use-automation-library-data.ts';
import { useAutomationRecordSession } from '../model/use-automation-record-session.ts';
import AutomationToggleDialog from '@/features/manage-automations/ui/AutomationToggleDialog.vue';
import AutomationRecordEditor from './AutomationRecordEditor.vue';
import { storeToRefs } from 'pinia';
import { computed, inject, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { routeLocationKey, routerKey } from 'vue-router';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import { useAutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { useAutomationSyncAdapters } from '@/features/manage-automations/model/use-automation-sync-adapter.ts';
import SyncButton from '@/features/sync-changes/ui/SyncButton.vue';
import AutomationEditorReview from '@/features/edit-automation-record/ui/AutomationEditorReview.vue';
import AutomationEditNotice from '@/features/edit-automation-record/ui/AutomationEditNotice.vue';
import RuntimeGlobalsEditor from '@/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import { useVariableSyncAdapter } from '@/features/edit-runtime-globals/model/use-variable-sync-adapter.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import GuideCatalogButton from '@/features/guided-help/ui/GuideCatalogButton.vue';
import { registerGuideActions } from '@/features/guided-help/model/guide-actions.ts';
import {
  isRecordSort,
  type RecordSort,
} from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import HotkeyIndicator from '@/entities/automation/ui/HotkeyIndicator.vue';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import type { AppliedRuleTemplate, RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { isRuleTemplateTab, type RuleTemplateTab } from '@/features/apply-rule-template/model/template-tab.ts';
import RuleTemplateDialog from '@/features/apply-rule-template/ui/RuleTemplateDialog.vue';
import ProfileImportDialog from '@/features/manage-rule-profiles/ui/ProfileImportDialog.vue';
import ProfileToolbar from '@/features/manage-rule-profiles/ui/ProfileToolbar.vue';
import ActionTypeDialog from '@/features/pick-action-type/ui/ActionTypeDialog.vue';
import { type RuleProfile } from '@/entities/rule-template/lib/rule-profiles.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import { isLibraryTab, type LibraryArea, type LibraryExtraArea, type LibraryTab } from '@/widgets/automation-library/model/library-tab.ts';
import AutomationLibrary from '@/widgets/automation-library/ui/AutomationLibrary.vue';
import AutomationRunsPanel from '@/widgets/automation-runs/ui/AutomationRunsPanel.vue';

const { t } = useAutomationText();
const guideCapabilities = useCapabilityStore();
const automation = useAutomationControls();
// The template dialog asks for (source, context); the store signature is
// (source, refresh, pluginId, context).
const templateLoadOptions = (source: string, context?: JsonObject) =>
  automation.loadOptions(source, false, context);
const syncAdapters = useAutomationSyncAdapters();
const variableSync = useVariableSyncAdapter();
const library = libraryJournal(useControlClient());
const { leaderboard } = storeToRefs(usePointsStore());
const saved = loadViewState().behavior;

const tab = ref<LibraryTab>(isLibraryTab(saved?.tab) ? saved.tab : 'events');
// Other pages (and the old Settings link) open the variables with `?view=variables`; no record opens behind them.
const requestedView = inject(routeLocationKey, null)?.query.view;
const area = ref<LibraryArea>(requestedView === 'variables' ? 'variables' : tab.value);
const selected = ref<string | null>(requestedView === 'variables' ? null : saved?.selected ?? null);
// The Cloud library lives in the rule templates dialog; only the variables sit next to Events and Actions.
const areas: LibraryExtraArea[] = ['variables'];
const eventQuery = ref(saved?.eventQuery ?? '');
const actionQuery = ref(saved?.actionQuery ?? '');
const eventSort = ref<RecordSort>(isRecordSort(saved?.eventSort) ? saved.eventSort : 'name');
const actionSort = ref<RecordSort>(isRecordSort(saved?.actionSort) ? saved.actionSort : 'name');
const pickerOpen = ref(false);
const pickerQuery = ref('');
const templatesOpen = ref(false);
const templatesTab = ref<RuleTemplateTab>('templates');
// Saved links to `?view=cloud` (the old Cloud tab) open the templates dialog on its Cloud tab.
// Optional injections: the page also renders without a router (tests, embeds).
const route = inject(routeLocationKey, null);
const router = inject(routerKey, null);
function openTemplates(next: RuleTemplateTab = 'templates'): void {
  templatesTab.value = isRuleTemplateTab(next) ? next : 'templates';
  templatesOpen.value = true;
}
if (requestedView === 'cloud') openTemplates('cloud');
watch(templatesOpen, (isOpen) => { if (!isOpen && route?.query.view === 'cloud') void router?.replace({ query: {} }); });
const profileImportOpen = ref(false);
const retryingLibrary = ref(false);
async function retryLibrary(): Promise<void> {
  if (retryingLibrary.value) return;
  retryingLibrary.value = true;
  try { await automation.loadProfiles(); }
  finally { retryingLibrary.value = false; }
}
const isNarrow = useMediaQuery('(width < 900px)');

const session = useAutomationRecordSession(automation, { tab, area, selected }, library.state);
const {
  editor, discardOpen, confirmDiscard, toggle, observedDefinitionState, reviewOpen,
  editing, hostChanged, librarySelected, libraryTab,
  closeEditor, newEvent, newAction, saveEvent, saveAction, removeRecord, duplicateRecord, deleteFromLibrary, setEnabled,
  compare, useReviewedBase, switchProfile,
} = session;

// `?view=variables` is a live navigation state, not just setup input. The
// same component is reused when a guide/deep link only changes the query, so
// entering and leaving that URL must update the library area explicitly.
watch(() => route?.query.view, (view, previous) => {
  if (view === 'cloud') openTemplates('cloud');
  if (view === 'variables' && area.value !== 'variables') libraryTab.value = 'variables';
  else if (previous === 'variables' && view !== 'variables' && area.value === 'variables') libraryTab.value = tab.value;
});

watch(() => route?.query.createAction, (requested) => {
  if (typeof requested !== 'string' || !requested) return;
  pickerQuery.value = requested;
  pickerOpen.value = true;
  if (!router || !route) return;
  const query = { ...route.query };
  delete query.createAction;
  void router.replace({ query });
}, { immediate: true });
watch(pickerOpen, (open) => { if (!open) pickerQuery.value = ''; });
const snapshot = computed(() => automation.snapshot.value);
const deleteTarget = ref<{ kind: LibraryTab; id: string; name: string } | null>(null);
const deleteDialogOpen = computed({
  get: () => !!deleteTarget.value,
  set: (open) => { if (!open) deleteTarget.value = null; },
});
function requestDelete(kind: LibraryTab, id: string): void {
  const record = (kind === 'events' ? snapshot.value.events : snapshot.value.actions).find((entry) => entry.id === id);
  if (record) deleteTarget.value = { kind, id, name: record.name };
}
function confirmDeleteRecord(): void {
  const target = deleteTarget.value;
  deleteTarget.value = null;
  if (target) void deleteFromLibrary(target.kind, target.id);
}
const guideTaskState = () => {
  const owner = automation.sourceHost.value;
  const definition = observedDefinitionState.value;
  const libraryState = library.state.value;
  return {
    'automation.saved': Boolean(owner && definition && owner.hostId === definition.hostId && owner.hostInstanceId === definition.hostInstanceId &&
      definition.pending === null && definition.recent.some(record => record.outcome === 'succeeded' && record.receipt !== null &&
        (record.intent.operation === 'create' || record.intent.operation === 'update'))),
    'changes.reviewed': Boolean(owner && libraryState && owner.hostId === libraryState.hostId && owner.hostInstanceId === libraryState.hostInstanceId &&
      libraryState.pending === null && libraryState.recent.some(record => record.outcome === 'succeeded' && record.receipt !== null)),
  };
};

const { packs, events, actions, eventTypes, availableTypes, gifts, viewers } = useAutomationLibraryData(automation, leaderboard);
function onSheetKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape' || event.defaultPrevented) return;
  if (!isNarrow.value || !editing.value) return;
  if (discardOpen.value || deleteDialogOpen.value || pickerOpen.value || templatesOpen.value || profileImportOpen.value || reviewOpen.value || toggle.open.value) return;
  closeEditor();
}

// Other pages link here with `?trigger=<event type>` to start a rule for it.
function openRequestedTrigger(): void {
  const trigger = route?.query.trigger;
  if (typeof trigger !== 'string' || !/^[a-z][a-z0-9_.-]{0,127}$/.test(trigger)) return;
  newEvent(trigger);
  void router?.replace({ query: {} });
}

function applyTemplate(actions: LiveAction[], event: LiveEvent, done: (success: boolean) => void): void {
  void automation.applyTemplate(actions, event).then(() => done(true), () => done(false));
}
function createProfile(name: string): void { void automation.createProfile(name).catch(() => {}); }
function deleteProfile(id: string): void { void automation.deleteProfile(id).catch(() => {}); }
function importTemplates(templates: RuleTemplate[], done: (added: number) => void): void {
  void automation.importTemplates(templates).then(done, () => done(0));
}

function applyProfile(profile: RuleProfile, applied: AppliedRuleTemplate[]): void {
  void automation.applyProfile(profile, applied).then(() => {
    if (!automation.profileError.value) profileImportOpen.value = false;
  }).catch(() => {});
}

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
// A pending debounced write must not outlive the page: after unmount the refs
// freeze with the editor still open, and the late write would restore a stale
// selection into whatever mounts next (in the real app, a remounted page; in
// tests, the next mount sharing the storage).
onBeforeUnmount(() => persist.cancel());

const guideScope = useTemplateRef<HTMLElement>('guideScope');
let unregisterGuideActions: (() => void) | undefined;

/** Open only an existing action that satisfies the specialized guide's type
 * contract. Returning false when none exists is deliberate: the tour must end
 * as unavailable rather than present an unrelated editor as if it were fetch,
 * media or TTS. */
function openMatchingGuideAction(matches: (action: LiveAction) => boolean): boolean {
  if (editor.dirty.value) return false;
  const current = editing.value;
  if (current?.kind === 'action' && matches(current.record)) return false;
  if (editing.value) { closeEditor(); return true; }
  if (area.value !== 'actions') { libraryTab.value = 'actions'; return true; }
  const match = actions.value.find(matches);
  if (!match) return false;
  if (!editing.value && selected.value === match.id) { closeEditor(); return true; }
  if (selected.value === match.id) return false;
  librarySelected.value = match.id;
  return true;
}

function guideActionTypeAvailable(typeId: string): boolean {
  return availableTypes.value.has(typeId);
}

function guideActionMatchesType(action: LiveAction, typeId: string): boolean {
  return action.typeId === typeId && guideActionTypeAvailable(typeId);
}

function guideActionMatchesTag(action: LiveAction, tag: string): boolean {
  const type = snapshot.value.actionTypes.find((candidate) => candidate.id === action.typeId);
  if (!type) return false;
  return type.tag === tag && guideActionTypeAvailable(type.id);
}

function guideActionTypeId(action: string, prefix: string): string {
  return action.startsWith(prefix) ? action.slice(prefix.length) : '';
}

function createGuideActionType(typeId: string): boolean {
  if (editor.dirty.value || !typeId || !guideActionTypeAvailable(typeId)) return false;
  const type = snapshot.value.actionTypes.find((candidate) => candidate.id === typeId);
  if (!type) return false;
  if (editing.value?.kind === 'action' && editing.value.record.typeId === typeId) return false;
  if (editing.value) { closeEditor(); return true; }
  if (area.value !== 'actions') { libraryTab.value = 'actions'; return true; }
  newAction(type);
  return true;
}

function openOrCreateGuideActionType(typeId: string): boolean {
  if (!typeId || !guideActionTypeAvailable(typeId)) return false;
  if (actions.value.some((action) => guideActionMatchesType(action, typeId))) {
    return openMatchingGuideAction((action) => guideActionMatchesType(action, typeId));
  }
  return createGuideActionType(typeId);
}

function revealGuideActionField(target: string, scope: Element | Document): boolean {
  const key = /^\[data-guide="action-field:([^"]+)"\]$/.exec(target)?.[1];
  if (!key) return false;
  const element = [...scope.querySelectorAll('[data-guide^="action-field:"]')]
    .find((candidate) => candidate.getAttribute('data-guide') === `action-field:${key}`);
  const toggle = element?.closest('.base-disclosure')?.querySelector('.base-disclosure__toggle');
  if (!(toggle instanceof HTMLElement) || toggle.getAttribute('aria-expanded') === 'true') return false;
  toggle.click();
  return true;
}

onMounted(() => {
  unregisterGuideActions = registerGuideActions('automations', {
    'automation.selectEvents': () => {
      if (editor.dirty.value || (!editing.value && area.value === 'events')) return false;
      libraryTab.value = 'events';
      return true;
    },
    'automation.selectActions': () => {
      if (editor.dirty.value || (!editing.value && area.value === 'actions')) return false;
      libraryTab.value = 'actions';
      return true;
    },
    'automation.selectVariables': () => {
      if (editor.dirty.value || (!editing.value && area.value === 'variables')) return false;
      libraryTab.value = 'variables';
      return true;
    },
    'automation.openEvent': () => {
      if (editor.dirty.value || editing.value?.kind === 'event') return false;
      if (editing.value) { closeEditor(); return true; }
      if (area.value !== 'events') { libraryTab.value = 'events'; return true; }
      const first = events.value[0];
      if (!first) return false;
      if (!editing.value && selected.value === first.id) { closeEditor(); return true; }
      if (selected.value === first.id) return false;
      librarySelected.value = first.id;
      return true;
    },
    'automation.openAction': () => {
      if (editor.dirty.value || editing.value?.kind === 'action') return false;
      if (editing.value) { closeEditor(); return true; }
      if (area.value !== 'actions') { libraryTab.value = 'actions'; return true; }
      const first = actions.value[0];
      if (!first) return false;
      if (!editing.value && selected.value === first.id) { closeEditor(); return true; }
      if (selected.value === first.id) return false;
      librarySelected.value = first.id;
      return true;
    },
    'automation.openActionType:core.fetch': () => openMatchingGuideAction((action) => guideActionMatchesType(action, 'core.fetch')),
    'automation.openActionType:audio.play': () => openMatchingGuideAction((action) => guideActionMatchesType(action, 'audio.play')),
    'automation.openActionTag:tts': () => openMatchingGuideAction((action) => guideActionMatchesTag(action, 'tts')),
    'automation.openActionType:*': ({ action }) => {
      const typeId = guideActionTypeId(action, 'automation.openActionType:');
      return typeId ? openMatchingGuideAction((record) => guideActionMatchesType(record, typeId)) : false;
    },
    'automation.createActionType:*': ({ action }) => createGuideActionType(guideActionTypeId(action, 'automation.createActionType:')),
    'automation.openOrCreateActionType:*': ({ action }) => openOrCreateGuideActionType(guideActionTypeId(action, 'automation.openOrCreateActionType:')),
    'automation.revealActionField': ({ target, scope }) => revealGuideActionField(target, scope),
  }, guideScope.value ?? undefined);
  openRequestedTrigger();
  void automation.loadCustomTemplates();
  window.addEventListener('keydown', onSheetKeydown);
});

onBeforeUnmount(() => {
  unregisterGuideActions?.();
  window.removeEventListener('keydown', onSheetKeydown);
});
</script>

<template>
  <div
    ref="guideScope"
    class="automations-page"
    data-guide-scope="automations"
  >
    <header class="automations-page__toolbar">
      <GuideCatalogButton
        :capabilities="guideCapabilities"
        :task-state="guideTaskState"
      />
      <ProfileToolbar
        data-guide="automation-profiles"
        :packs="packs"
        :active-id="automation.activeProfileId.value"
        :disabled="automation.profileLoading.value"
        @switch="switchProfile"
        @create="createProfile"
        @delete="deleteProfile"
        @export="automation.exportProfile"
        @import="profileImportOpen = true"
      />
      <span
        class="automations-page__divider"
        aria-hidden="true"
      />
      <BaseIconButton
        :label="t('automations.templates.open')"
        data-guide="automation-templates"
        variant="default"
        :disabled="automation.profileLoading.value"
        @click="openTemplates()"
      >
        <IconPlus />
      </BaseIconButton>
      <span class="automations-page__spacer" />
      <HotkeyIndicator
        :plugins="snapshot.plugins"
        :status="automation.hotkeyStatus.value"
        :last-event="automation.lastHotkeyEvent.value"
        :access-pending="automation.hotkeyAccessPending.value"
        @request-access="automation.requestHotkeyAccess()"
      />
    </header>
    <div
      v-if="automation.profileLoading.value"
      class="automations-page__banner"
      data-test="library-loading"
      role="status"
    >
      {{ t('libraryRecovery.loading') }}
      <BaseButton
        size="sm"
        :disabled="retryingLibrary"
        data-test="library-retry-loading"
        @click="retryLibrary"
      >
        {{ t('libraryRecovery.retryLoading') }}
      </BaseButton>
    </div>
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

    <!-- One navigation area: the library, or the open record with a way back. Runs stay below. -->
    <div class="automations-page__work">
      <div class="automations-page__nav">
        <AutomationLibrary
          v-if="!editing || isNarrow"
          v-model:tab="libraryTab"
          v-model:selected="librarySelected"
          v-model:event-query="eventQuery"
          v-model:action-query="actionQuery"
          v-model:event-sort="eventSort"
          v-model:action-sort="actionSort"
          data-guide="automation-library"
          :events="events"
          :actions="actions"
          :action-types="snapshot.actionTypes"
          :available-types="availableTypes"
          :event-types="eventTypes"
          :runs="automation.runs.value"
          :areas="areas"
          @new-event="newEvent"
          @new-action="pickerOpen = true"
          @set-enabled="setEnabled"
          @duplicate="duplicateRecord"
          @delete="requestDelete"
        >
          <template #sync="{ kind }">
            <SyncButton
              :key="kind"
              :adapter="kind === 'events' ? syncAdapters.event : syncAdapters.action"
            />
          </template>
          <template #variables>
            <RuntimeGlobalsEditor data-guide="automation-variables">
              <template #sync="{ dirty }">
                <SyncButton
                  v-if="variableSync.supported.value"
                  :adapter="variableSync.adapter"
                  :blocked-reason="dirty ? t('syncChanges.variablesSaveFirst') : undefined"
                />
              </template>
            </RuntimeGlobalsEditor>
          </template>
        </AutomationLibrary>
        <div
          v-if="editing"
          class="automations-page__sheet"
          :role="isNarrow ? 'dialog' : 'region'"
          :aria-modal="isNarrow ? 'true' : undefined"
          :aria-label="editing.record.name"
        >
          <div class="automations-page__sheet-body">
            <AutomationEditNotice
              :edit="editor.state.value"
              :host-changed="hostChanged"
              :memory="automation.persistent.value === false"
            />
            <AutomationRecordEditor
              data-guide="automation-editor"
              :editing="editing"
              :state="editor.state.value"
              :save-blocked="editor.saveBlocked.value"
              :snapshot="snapshot"
              :event-types="eventTypes"
              :actions="actions"
              :gifts="gifts"
              :viewers="viewers"
              :automation="automation"
              @save-event="saveEvent"
              @save-action="saveAction"
              @compare="compare"
              @change="editor.change"
              @remove="removeRecord"
              @close="closeEditor"
            />
          </div>
        </div>
      </div>
      <AutomationRunsPanel
        data-guide="automation-runs"
        class="automations-page__runs"
        :runs="automation.runs.value"
      />
    </div>

    <ActionTypeDialog
      v-model:open="pickerOpen"
      :plugins="snapshot.plugins"
      :action-types="snapshot.actionTypes"
      :initial-query="pickerQuery"
      @pick="newAction"
      @open-plugins="automation.openPlugins()"
    />
    <RuleTemplateDialog
      v-model:open="templatesOpen"
      v-model:tab="templatesTab"
      :snapshot="snapshot"
      :custom-templates="automation.customTemplates.value"
      :custom-error="automation.templateError.value"
      :working="library.working.value || library.gate.busy.value"
      :action-options="automation.actionOptions.value"
      :load-options="templateLoadOptions"
      @apply="applyTemplate"
      @import="importTemplates"
      @delete-custom="automation.deleteCustomTemplate"
    />
    <ProfileImportDialog
      v-model:open="profileImportOpen"
      :error="automation.profileError.value"
      :working="library.working.value || library.gate.busy.value"
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
    <BaseConfirmDialog
      v-model:open="deleteDialogOpen"
      :title="t('automations.library.deleteTitle', { name: deleteTarget?.name ?? '' })"
      :message="t('automations.library.deleteMessage', { name: deleteTarget?.name ?? '' })"
      :confirm-label="t('automations.library.delete')"
      danger
      @confirm="confirmDeleteRecord"
    />
    <AutomationEditorReview
      v-model:open="reviewOpen"
      :edit="editor.state.value"
      :review="editor.review.value"
      :busy="editor.reading.value"
      :error="editor.readError.value"
      :can-adopt="editor.canAdoptReview.value"
      @refresh="editor.readForReview"
      @adopt="useReviewedBase"
    />
    <AutomationToggleDialog
      :toggle="toggle"
      @update:open="toggle.open.value = $event"
    />
  </div>
</template>

<style src="./automations-page.css"></style>
