import { computed, onBeforeUnmount, ref, shallowRef, watch, type Ref } from 'vue';
import type { ActionTypeDefinition, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { useAutomationToggleReview } from '@/features/manage-automations/model/use-automation-toggle-review.ts';
import { useUnsavedSelectionGuard } from '@/features/manage-automations/model/use-unsaved-selection-guard.ts';
import { useAutomationEditor } from '@/features/edit-automation-record/model/automation-editor.ts';
import type { DefinitionState } from '@/entities/automation/model/definition-contracts.ts';
import type { LibraryState } from '@/entities/rule-template/model/library-contracts.ts';
import { recordKey } from '@/entities/automation/model/record-versions.ts';
import { createActionFromType, createEvent } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { isLibraryTab, type LibraryArea, type LibraryTab } from '@/widgets/automation-library/model/library-tab.ts';
import type { EditingRecord } from './editing-record.ts';

type RecordSessionControls = Pick<AutomationControls,
  'snapshot' | 'persistent' | 'sourceHost' | 'revisionFor' | 'readRecord' |
  'saveEvent' | 'saveAction' | 'deleteRecord' | 'setRecordEnabled' | 'switchProfile'>;

/** Owns the page-lifetime record draft, navigation guard and revision review. */
export function useAutomationRecordSession(
  automation: RecordSessionControls,
  selection: { tab: Ref<LibraryTab>; area: Ref<LibraryArea>; selected: Ref<string | null> },
  libraryState: Readonly<Ref<LibraryState | null>>,
) {
  const { t } = useAutomationText();
  const { tab, area, selected } = selection;
  const snapshot = automation.snapshot;
  const reviewOpen = ref(false);
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
  watch(libraryState, (next, previous) => {
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

  onBeforeUnmount(editor.dispose);
  return {
    editor, discardOpen, confirmDiscard, toggle, observedDefinitionState,
    reviewOpen, editing, proposalTarget, proposalVersions, hostChanged,
    librarySelected, libraryTab, closeEditor, newEvent, newAction,
    saveEvent, saveAction, removeRecord, setEnabled, compare,
    recoveredDefinition, useReviewedBase, switchProfile,
  };
}
