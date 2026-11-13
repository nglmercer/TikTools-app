<script setup lang="ts">
import { useAutomationHistory } from '@/widgets/automation-history-menu/model/use-automation-history.ts';
import { useAutomationLibraryData } from '../model/use-automation-library-data.ts';
import { useAutomationRecordSession } from '../model/use-automation-record-session.ts';
import AutomationSheetToolbar from './AutomationSheetToolbar.vue';
import AutomationToggleDialog from '@/features/manage-automations/ui/AutomationToggleDialog.vue';
import AutomationRecordEditor from './AutomationRecordEditor.vue';
import { storeToRefs } from 'pinia';
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { routeLocationKey, routerKey } from 'vue-router';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { useAutomationSyncAdapters } from '@/features/manage-automations/model/use-automation-sync-adapter.ts';
import SyncButton from '@/features/sync-changes/ui/SyncButton.vue';
import AutomationEditorReview from '@/features/edit-automation-record/ui/AutomationEditorReview.vue';
import AutomationEditNotice from '@/features/edit-automation-record/ui/AutomationEditNotice.vue';
import AutomationProposalReview from '@/features/review-automation-proposal/ui/AutomationProposalReview.vue';
import AutomationApprovalReview from '@/features/review-automation-approval/ui/AutomationApprovalReview.vue';
import RuntimeGlobalsEditor from '@/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import { useVariableSyncAdapter } from '@/features/edit-runtime-globals/model/use-variable-sync-adapter.ts';
import OwnerLibraryChanges from '@/features/recover-owner-library/ui/OwnerLibraryChanges.vue';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import GuideButton from '@/features/guided-help/ui/GuideButton.vue';
import OwnerDefinitionRecovery from '@/features/recover-owner-definition/ui/OwnerDefinitionRecovery.vue';
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
import AutomationHistoryMenu from '@/widgets/automation-history-menu/ui/AutomationHistoryMenu.vue';
import AutomationLibrary from '@/widgets/automation-library/ui/AutomationLibrary.vue';
import AutomationRunsPanel from '@/widgets/automation-runs/ui/AutomationRunsPanel.vue';

const { t } = useAutomationText();
const guideCapabilities = useCapabilityStore();
const automation = useAutomationControls();
// Variables sync is a TODO (docs/SYNC-CHANGES-PLAN.md): register its adapter here and add a button to that tab once a Cloud collection carries it.
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

const session = useAutomationRecordSession(automation, { tab, area, selected }, library.state);
const {
  editor, discardOpen, confirmDiscard, toggle, observedDefinitionState, reviewOpen,
  editing, proposalTarget, proposalVersions, hostChanged, librarySelected, libraryTab,
  closeEditor, newEvent, newAction, saveEvent, saveAction, removeRecord, setEnabled,
  compare, recoveredDefinition, useReviewedBase, switchProfile,
} = session;
const history = useAutomationHistory(automation, () => proposalTarget.value);
const { historyItems, historyAttention, openHistory } = history;
const snapshot = computed(() => automation.snapshot.value);
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
  if (discardOpen.value || pickerOpen.value || templatesOpen.value || profileImportOpen.value || reviewOpen.value || proposalReviewOpen.value || approvalReviewOpen.value || toggle.open.value) return;
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

onMounted(() => {
  openRequestedTrigger();
  void automation.loadCustomTemplates();
  window.addEventListener('keydown', onSheetKeydown);
});

onBeforeUnmount(() => { window.removeEventListener('keydown', onSheetKeydown); });
</script>

<template>
  <div
    class="automations-page"
    data-guide-scope="automations"
  >
    <header class="automations-page__toolbar">
      <GuideButton
        tour="automation"
        :capabilities="guideCapabilities"
        :task-state="guideTaskState"
      />
      <GuideButton
        tour="changes"
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
      <AutomationProposalReview
        v-if="automation.persistent.value === true"
        :ref="history.proposalReviewPanel"
        v-model:open="proposalReviewOpen"
        hide-triggers
        :target="proposalTarget"
        :host-versions="proposalVersions"
      />
      <OwnerDefinitionRecovery
        v-if="automation.persistent.value === true || automation.sourceHost.value !== null"
        :ref="history.definitionRecoveryPanel"
        hide-triggers
        :selection-key="`${tab}:${selected ?? ''}:${editor.state.value?.candidate.id ?? ''}`"
        @recovered="recoveredDefinition"
        @observed="observedDefinitionState = $event"
      />
      <OwnerLibraryChanges
        v-if="automation.persistent.value !== false"
        :ref="history.libraryPanel"
        hide-triggers
        :selection-key="`${tab}:${selected ?? ''}:${editor.state.value?.candidate.id ?? ''}`"
        @recovered="recoveredDefinition"
      />
      <AutomationApprovalReview
        v-if="automation.persistent.value === true"
        :ref="history.approvalReviewPanel"
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
        >
          <template #sync="{ kind }">
            <SyncButton
              :key="kind"
              :adapter="kind === 'events' ? syncAdapters.event : syncAdapters.action"
            />
          </template>
          <template #variables>
            <RuntimeGlobalsEditor>
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
          <!-- On narrow screens the sheet covers the toolbar, so it carries its own bar with the record tools. -->
          <AutomationSheetToolbar
            v-if="isNarrow"
            :editing="editing"
            :automation="automation"
            :proposal-target="proposalTarget"
            :history="history"
            @close="closeEditor"
          />
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
    <AutomationToggleDialog
      :toggle="toggle"
      @update:open="toggle.open.value = $event"
    />
  </div>
</template>

<style src="./automations-page.css"></style>
