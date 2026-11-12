<script setup lang="ts">
import { useAutomationHistory } from '@/widgets/automation-history-menu/model/use-automation-history.ts';
import { useAutomationLibraryData } from '../model/use-automation-library-data.ts';
import type { EditingRecord } from '../model/editing-record.ts';
import AutomationRecordEditor from './AutomationRecordEditor.vue';
import { storeToRefs } from 'pinia';
import { computed, inject, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import { routeLocationKey, routerKey } from 'vue-router';
import type { ActionTypeDefinition, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { useAutomationSyncAdapters } from '@/features/manage-automations/model/use-automation-sync-adapter.ts';
import { useAutomationToggleReview } from '@/features/manage-automations/model/use-automation-toggle-review.ts';
import { useUnsavedSelectionGuard } from '@/features/manage-automations/model/use-unsaved-selection-guard.ts';
import SyncButton from '@/features/sync-changes/ui/SyncButton.vue';
import { useAutomationEditor } from '@/features/edit-automation-record/model/automation-editor.ts';
import AutomationEditorReview from '@/features/edit-automation-record/ui/AutomationEditorReview.vue';
import AutomationEditNotice from '@/features/edit-automation-record/ui/AutomationEditNotice.vue';
import AutomationProposalReview from '@/features/review-automation-proposal/ui/AutomationProposalReview.vue';
import AutomationApprovalReview from '@/features/review-automation-approval/ui/AutomationApprovalReview.vue';
import RuntimeGlobalsEditor from '@/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import OwnerLibraryChanges from '@/features/recover-owner-library/ui/OwnerLibraryChanges.vue';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import OwnerDefinitionRecovery from '@/features/recover-owner-definition/ui/OwnerDefinitionRecovery.vue';
import type { DefinitionState } from '@/entities/automation/model/definition-contracts.ts';
import { recordKey } from '@/entities/automation/model/record-versions.ts';
import {
  createActionFromType,
  createEvent,
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
import { type RuleProfile } from '@/entities/rule-template/model/rule-profiles-state.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import { isLibraryTab, type LibraryArea, type LibraryExtraArea, type LibraryTab } from '@/widgets/automation-library/model/library-tab.ts';
import AutomationHistoryMenu from '@/widgets/automation-history-menu/ui/AutomationHistoryMenu.vue';
import AutomationLibrary from '@/widgets/automation-library/ui/AutomationLibrary.vue';
import AutomationRunsPanel from '@/widgets/automation-runs/ui/AutomationRunsPanel.vue';

const { t } = useAutomationText();
const automation = useAutomationControls();
// Variables sync is a TODO (docs/SYNC-CHANGES-PLAN.md): register its adapter here and add a button to that tab once a Cloud collection carries it.
const syncAdapters = useAutomationSyncAdapters();
const library = libraryJournal(useControlClient());
const editor = useAutomationEditor({
  read: automation.readRecord,
  save: (kind, record, base) => kind === 'event'
    ? automation.saveEvent(record as LiveEvent, base) : automation.saveAction(record as LiveAction, base),
  remove: automation.deleteRecord,
});
// Selection with unsaved changes and the enable/disable review are coordinated by two composables.
const { discardOpen, guard, confirmDiscard } = useUnsavedSelectionGuard(editor.dirty);
const toggle = useAutomationToggleReview({
  controls: automation,
  findRecord: (kind, id) => (kind === 'event' ? snapshot.value.events : snapshot.value.actions).find((record) => record.id === id),
  t,
});
watch(library.state, (next, previous) => {
  if (JSON.stringify(next) !== JSON.stringify(previous)) {
    editor.requireRecoveryReview();
    toggle.markNeedsReview();
  }
});
const observedDefinitionState = shallowRef<DefinitionState | null>(null);
watch([editor.state, automation.sourceHost, observedDefinitionState], ([edit, host, journal]) => {
  if (!edit || edit.reviewRequired) return;
  if (edit.host && [host, journal].some((current) => current &&
    (current.hostId !== edit.host!.hostId || current.hostInstanceId !== edit.host!.hostInstanceId)) ||
    typeof edit.revision === 'string' && journal?.recent.some((record) => record.intent.kind === edit.kind && record.intent.id === edit.candidate.id &&
      record.receipt && BigInt(record.receipt.revision) > BigInt(edit.revision!))) editor.requireRecoveryReview();
}, { flush: 'sync' });
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
const reviewOpen = ref(false);
const proposalReviewOpen = ref(false);
const approvalReviewOpen = ref(false);
const pickerOpen = ref(false);
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
watch(() => route?.query.view, (view) => { if (view === 'cloud') openTemplates('cloud'); });
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

const { libraryPanel, proposalReviewPanel, definitionRecoveryPanel, approvalReviewPanel, historyItems, historyAttention, openHistory } = useAutomationHistory(automation, () => proposalTarget.value);
const snapshot = computed(() => automation.snapshot.value);

const { packs, events, actions, eventTypes, availableTypes, gifts, viewers } = useAutomationLibraryData(automation, leaderboard);
const editing = computed<EditingRecord | null>(() => {
  const state = editor.state.value;
  if (!state) return null;
  return state.kind === 'event' ? { kind: 'event', record: state.candidate as LiveEvent, isNew: state.create }
    : { kind: 'action', record: state.candidate as LiveAction, isNew: state.create };
});
const proposalTarget = computed(() => {
  const state = editor.state.value;
  if (!state || state.create) return null;
  const rows = state.kind === 'event' ? snapshot.value.events : snapshot.value.actions;
  return rows.some((row) => row.id === state.candidate.id) ? { kind: state.kind, id: state.candidate.id } : null;
});
const proposalVersions = computed(() => {
  const versions = new Map<string, string>();
  for (const [kind, rows] of [['event', snapshot.value.events], ['action', snapshot.value.actions]] as const) {
    for (const row of rows) {
      const revision = automation.revisionFor(kind, row.id);
      if (typeof revision === 'string') versions.set(recordKey(kind, row.id), revision);
    }
  }
  return versions;
});
const hostChanged = computed(() => {
  const state = editor.state.value;
  if (!state || state.create || state.revision === null || state.revision === undefined) return false;
  const revision = automation.revisionFor(state.kind, state.candidate.id);
  return revision === undefined || (typeof revision === 'string' && BigInt(revision) > BigInt(state.revision));
});

function resetEditor(): void { editor.close(); reviewOpen.value = false; }

const librarySelected = computed<string | null>({
  get: () => selected.value,
  set: (next) => { if (next === selected.value) return; guard(() => {
    resetEditor();
    selected.value = next;
  }); },
});
const libraryTab = computed<LibraryArea>({
  get: () => area.value,
  set: (next) => guard(() => {
    resetEditor();
    if (isLibraryTab(next)) tab.value = next;
    area.value = next;
    selected.value = null;
  }),
});

function closeEditor(): void {
  guard(() => {
    resetEditor();
    selected.value = null;
  });
}

function onSheetKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape' || event.defaultPrevented) return;
  if (!isNarrow.value || !editing.value) return;
  if (discardOpen.value || pickerOpen.value || templatesOpen.value || profileImportOpen.value || reviewOpen.value || proposalReviewOpen.value || approvalReviewOpen.value || toggle.open.value) return;
  closeEditor();
}

function newEvent(trigger?: string): void {
  guard(() => {
    tab.value = 'events';
    area.value = 'events';
    const record = createEvent(t('automations.editor.newEventName'));
    if (trigger) record.trigger = trigger;
    selected.value = record.id;
    editor.open('event', record, automation.persistent.value === false ? null : '0', true, automation.sourceHost.value);
  });
}

// Other pages link here with `?trigger=<event type>` to start a rule for it.
function openRequestedTrigger(): void {
  const trigger = route?.query.trigger;
  if (typeof trigger !== 'string' || !/^[a-z][a-z0-9_.-]{0,127}$/.test(trigger)) return;
  newEvent(trigger);
  void router?.replace({ query: {} });
}

function newAction(type: ActionTypeDefinition): void {
  guard(() => {
    tab.value = 'actions';
    area.value = 'actions';
    const record = createActionFromType(type, localizedText);
    selected.value = record.id;
    editor.open('action', record, automation.persistent.value === false ? null : '0', true, automation.sourceHost.value);
  });
}

function saveEvent(event: LiveEvent): void {
  editor.change(event);
  void editor.save();
}

function saveAction(action: LiveAction): void {
  editor.change(action);
  void editor.save();
}

function removeRecord(kind: LibraryTab, id: string): void {
  if (editing.value?.record.id !== id || editing.value.kind !== (kind === 'events' ? 'event' : 'action')) return;
  void editor.remove().then(() => {
    if (!editor.state.value && selected.value === id && tab.value === kind) selected.value = null;
  });
}

function setEnabled(kind: LibraryTab, id: string, enabled: boolean): void {
  toggle.request(kind === 'events' ? 'event' : 'action', id, enabled);
}

function compare(): void {
  reviewOpen.value = true;
  void editor.readForReview();
}
function recoveredDefinition(): void {
  editor.requireRecoveryReview();
  toggle.recovered(t('definitionRecovery.compare'));
}
function useReviewedBase(): void {
  if (editor.adoptReview()) reviewOpen.value = false;
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

function switchProfile(id: string): void {
  guard(() => {
    resetEditor();
    selected.value = null;
    void automation.switchProfile(id).catch(() => {});
  });
}

watch(
  () => [selected.value, tab.value, snapshot.value.events, snapshot.value.actions] as const,
  () => {
    if (editor.state.value || !selected.value) return;
    const kind = tab.value === 'events' ? 'event' : 'action';
    const record = (kind === 'event' ? snapshot.value.events : snapshot.value.actions).find((r) => r.id === selected.value);
    if (record) editor.open(kind, record, automation.revisionFor(kind, record.id), false, automation.sourceHost.value);
  },
  { immediate: true },
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

onBeforeUnmount(() => { editor.dispose(); window.removeEventListener('keydown', onSheetKeydown); });
</script>

<template>
  <div class="automations-page">
    <header class="automations-page__toolbar">
      <ProfileToolbar
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
        variant="default"
        :disabled="automation.profileLoading.value"
        @click="openTemplates()"
      >
        <IconPlus />
      </BaseIconButton>
      <AutomationProposalReview
        v-if="automation.persistent.value === true"
        ref="proposalReviewPanel"
        v-model:open="proposalReviewOpen"
        hide-triggers
        :target="proposalTarget"
        :host-versions="proposalVersions"
      />
      <OwnerDefinitionRecovery
        v-if="automation.persistent.value === true || automation.sourceHost.value !== null"
        ref="definitionRecoveryPanel"
        hide-triggers
        :selection-key="`${tab}:${selected ?? ''}:${editor.state.value?.candidate.id ?? ''}`"
        @recovered="recoveredDefinition"
        @observed="observedDefinitionState = $event"
      />
      <OwnerLibraryChanges
        v-if="automation.persistent.value !== false"
        ref="libraryPanel"
        hide-triggers
        :selection-key="`${tab}:${selected ?? ''}:${editor.state.value?.candidate.id ?? ''}`"
        @recovered="recoveredDefinition"
      />
      <AutomationApprovalReview
        v-if="automation.persistent.value === true"
        ref="approvalReviewPanel"
        v-model:open="approvalReviewOpen"
        hide-triggers
        :target="proposalTarget"
        :host-versions="proposalVersions"
      />
      <AutomationHistoryMenu
        v-if="historyItems.length"
        :items="historyItems"
        :attention="historyAttention"
        @select="openHistory"
      />
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
        >
          <template #sync="{ kind }">
            <SyncButton
              :key="kind"
              :adapter="kind === 'events' ? syncAdapters.event : syncAdapters.action"
            />
          </template>
          <template #variables>
            <RuntimeGlobalsEditor />
          </template>
        </AutomationLibrary>
        <div
          v-if="editing"
          class="automations-page__sheet"
          :role="isNarrow ? 'dialog' : 'region'"
          :aria-modal="isNarrow ? 'true' : undefined"
          :aria-label="editing.record.name"
        >
          <!-- On narrow screens the sheet covers the toolbar, so it carries its own bar with the record tools. -->
          <header
            v-if="isNarrow"
            class="automations-page__sheet-bar"
          >
            <BaseIconButton
              :label="t('common.back')"
              variant="default"
              size="sm"
              @click="closeEditor"
            >
              <IconChevronLeft />
            </BaseIconButton>
            <span class="automations-page__sheet-title">{{ editing.record.name }}</span>
            <template v-if="isNarrow">
              <BaseButton
                v-if="automation.persistent.value !== false"
                size="sm"
                :disabled="!libraryPanel || libraryPanel.busy"
                data-test="library-history"
                @click="libraryPanel?.openHistory()"
              >
                {{ t(libraryPanel?.hasPending ? 'libraryRecovery.pendingButton' : 'libraryRecovery.history') }}
              </BaseButton>
              <BaseButton
                v-if="automation.persistent.value === true || automation.sourceHost.value !== null"
                size="sm"
                :disabled="!definitionRecoveryPanel || definitionRecoveryPanel.busy"
                data-test="definition-history"
                @click="definitionRecoveryPanel?.openHistory()"
              >
                {{ t(definitionRecoveryPanel?.hasPending ? 'definitionRecovery.pendingButton' : 'definitionRecovery.history') }}
              </BaseButton>
              <BaseButton
                v-if="automation.persistent.value === true && proposalTarget"
                class="automations-page__proposal-button"
                size="sm"
                @click="proposalReviewPanel?.openReview()"
              >
                {{ t('automationProposal.open') }}
              </BaseButton>
              <BaseButton
                v-if="automation.persistent.value === true && proposalTarget"
                size="sm"
                data-test="approval-open"
                @click="approvalReviewPanel?.openReview()"
              >
                {{ t('automationApproval.open') }}
              </BaseButton>
            </template>
          </header>
          <div class="automations-page__sheet-body">
            <AutomationEditNotice
              :edit="editor.state.value"
              :host-changed="hostChanged"
              :memory="automation.persistent.value === false"
              @compare="compare"
            />
            <AutomationRecordEditor
              :editing="editing"
              :state="editor.state.value"
              :save-blocked="editor.saveBlocked.value"
              :is-narrow="isNarrow"
              :snapshot="snapshot"
              :event-types="eventTypes"
              :actions="actions"
              :gifts="gifts"
              :viewers="viewers"
              :automation="automation"
              @save-event="saveEvent"
              @save-action="saveAction"
              @change="editor.change"
              @remove="removeRecord"
              @close="closeEditor"
            />
          </div>
        </div>
      </div>
      <AutomationRunsPanel
        class="automations-page__runs"
        :runs="automation.runs.value"
      />
    </div>

    <ActionTypeDialog
      v-model:open="pickerOpen"
      :plugins="snapshot.plugins"
      :action-types="snapshot.actionTypes"
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
    <BaseConfirmDialog
      v-model:open="toggle.open.value"
      :title="t('automations.conflict.toggleTitle')"
      :message="t(toggle.target.value?.enabled ? 'automations.conflict.enableMessage' : 'automations.conflict.disableMessage', { name: toggle.target.value?.name ?? '' })"
      :confirm-label="t('automations.conflict.confirmToggle')"
      :busy="toggle.confirmBlocked.value"
      @confirm="toggle.confirm"
    >
      <p class="automations-page__target">
        {{ t('automations.conflict.recordVersion', { id: toggle.target.value?.id ?? '', revision: toggle.target.value?.revision ?? t('automations.conflict.memory') }) }}
      </p>
      <p
        v-if="toggle.error.value"
        role="alert"
      >
        {{ toggle.error.value }}
      </p>
      <BaseButton
        v-if="toggle.needsReview.value"
        :disabled="toggle.busy.value"
        @click="toggle.review"
      >
        {{ t('automations.conflict.readCurrent') }}
      </BaseButton>
    </BaseConfirmDialog>
    <p
      v-if="toggle.error.value && !toggle.open.value"
      class="automations-page__banner automations-page__banner--error"
      role="alert"
    >
      {{ toggle.error.value }}
    </p>
  </div>
</template>

<style src="./automations-page.css"></style>
