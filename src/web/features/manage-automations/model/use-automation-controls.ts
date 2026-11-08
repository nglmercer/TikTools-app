import { storeToRefs } from 'pinia';
import type { Ref } from 'vue';
import type { BehaviorRun, BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import type { GiftCatalogEntry, HotkeyStatusData } from '../../../../shared/messages.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useRuleProfilesStore } from '@/entities/rule-template/model/rule-profiles-store.ts';
import type { ProfilePack } from '@/entities/rule-template/model/rule-profiles-state.ts';
import type { RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { useRuleTemplatesStore } from '@/entities/rule-template/model/rule-templates-store.ts';
import { useGlobalsStore } from '@/entities/runtime-global/model/globals-store.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import type { LastHotkeyEvent } from '@/entities/automation/model/automation-state.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';

export function useAutomationControls() {
  const automation = useAutomationStore();
  const plugins = usePluginsStore();
  const templates = useRuleTemplatesStore();
  const profiles = useRuleProfilesStore();
  const navigation = useNavigationStore();
  const state = storeToRefs(automation);
  const pluginRefs = storeToRefs(plugins);
  const templateRefs = storeToRefs(templates);
  const profileRefs = storeToRefs(profiles);
  const { giftCatalog } = storeToRefs(useLiveFeedStore());
  const { globals } = storeToRefs(useGlobalsStore());

  return {
    snapshot: state.behavior as unknown as Readonly<Ref<BehaviorSnapshot>>,
    runs: state.behaviorRuns as unknown as Readonly<Ref<readonly BehaviorRun[]>>,
    testRuns: state.behaviorTestRuns as unknown as Readonly<Ref<readonly BehaviorRun[]>>,
    error: state.behaviorError,
    persistent: state.persistent,
    revisionFor: automation.revisionFor,
    readRecord: automation.readRecord,
    deleteRecord: automation.deleteRecord,
    setRecordEnabled: automation.setRecordEnabled,
    gifts: giftCatalog as unknown as Readonly<Ref<readonly GiftCatalogEntry[]>>,
    globals,
    hotkeyStatus: state.hotkeyStatus as unknown as Readonly<Ref<HotkeyStatusData | null>>,
    lastHotkeyEvent: state.lastHotkeyEvent as unknown as Readonly<Ref<LastHotkeyEvent | null>>,
    hotkeyAccessPending: state.hotkeyAccessPending,
    actionOptions: pluginRefs.actionOptions,
    actionOptionErrors: pluginRefs.actionOptionErrors,
    customTemplates: templateRefs.custom as unknown as Readonly<Ref<readonly RuleTemplate[]>>,
    templateError: templateRefs.error,
    profilePacks: profileRefs.packs as unknown as Readonly<Ref<readonly ProfilePack[]>>,
    activeProfileId: profileRefs.activeId,
    profileError: profileRefs.error,
    profileNotice: profileRefs.notice,
    openMediaPicker: plugins.openMediaPicker,
    openPlugins: () => navigation.open('plugins'),
    requestHotkeyAccess: automation.handleRequestHotkeyAccess,
    saveAction: profiles.saveAction,
    deleteAction: automation.handleDeleteAction,
    setActionEnabled: automation.handleSetActionEnabled,
    testAction: automation.handleTestAction,
    saveEvent: profiles.saveEvent,
    deleteEvent: automation.handleDeleteEvent,
    setEventEnabled: automation.handleSetEventEnabled,
    testEvent: automation.handleTestEvent,
    fireEvent: automation.handleFireEvent,
    loadOptions: plugins.handleGetActionOptions,
    applyTemplate: automation.handleApplyRuleTemplate,
    loadCustomTemplates: templates.loadCustom,
    importTemplates: templates.importTemplates,
    deleteCustomTemplate: templates.deleteCustom,
    applyProfile: profiles.applyProfile,
    switchProfile: profiles.switchProfile,
    createProfile: profiles.createProfile,
    deleteProfile: profiles.deleteProfile,
    exportProfile: profiles.exportProfile,
  };
}

export type AutomationControls = ReturnType<typeof useAutomationControls>;
