import { onMounted, onUnmounted, ref, watch } from 'vue';

import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import { parsePluginNavId } from '../../automation/plugins/declarative.ts';
import { useAnalytics } from '../features/analytics.ts';
import { useAutomation } from '../features/automation.ts';
import { useConnection } from '../features/connection.ts';
import { useCreators } from '../features/creators.ts';
import { useLive } from '../features/live.ts';
import { useMedia } from '../features/media.ts';
import { usePlugins } from '../features/plugins.ts';
import { usePoints } from '../features/points.ts';
import { useProcessors } from '../features/processors.ts';
import { useWidgets } from '../features/widgets.ts';
import { useRuleTemplates } from '../features/rule-templates.ts';
import {
  buildProfileDoc,
  downloadTextFile,
  resolvedPacks,
  useRuleProfiles,
  type RuleProfile,
} from '../features/rule-profiles.ts';
import { useGlobals } from '../features/globals.ts';
import { createControlClient, errorMessage } from '../platform/control-client.ts';
import type { AppliedRuleTemplate } from '../views/behavior/rule-templates.ts';
import {
  controlBackend,
  type BrokerBackend,
} from '../plugin-ui/plugin-webview-host.ts';
import {
  applyTheme,
  getInitialLocale,
  getInitialTheme,
  saveLocale,
  saveTheme,
  type Theme,
} from '../preferences.ts';
import { t, type Locale } from '../i18n.ts';
import type { AppTab } from '../types.ts';
import { createInitialPluginInstallState, pluginPickerOptions } from './plugin-install.ts';

export type { PluginInstallState } from './plugin-install.ts';
export { defaultPointsConfig } from '../features/points.ts';

export const initialPluginInstallState = createInitialPluginInstallState();

const initialLocale = getInitialLocale();
const initialTheme = getInitialTheme();

/**
 * Composition root: one shared JSON-RPC control client plus one
 * composable per domain. Domain state and host calls live in
 * `src/web/features/`; this module only wires cross-feature callbacks,
 * owns the tab/locale/theme chrome, and preserves the controller API
 * App.vue consumes.
 */
export function useAppController() {
  const control = createControlClient();
  const activeTab = ref<AppTab>('feed');
  const locale = ref<Locale>(initialLocale);
  const theme = ref<Theme>(initialTheme);

  const translate = (key: string): string => t(locale.value, key);

  applyTheme(initialTheme);
  document.documentElement.lang = initialLocale;

  const connection = useConnection(control, {
    goFeed: () => {
      activeTab.value = 'feed';
    },
    resetFeed: () => live.resetEvents(),
    translate,
  });
  const points = usePoints(control);
  const creators = useCreators(control, {
    noteCreatorSeen: (clean, persist) => connection.noteCreatorSeen(clean, persist),
    mergeRecentNames: (names) => connection.mergeRecentNames(names),
  });
  const analytics = useAnalytics(control, () => connection.activeCreator.value);
  const automation = useAutomation(control);
  const widgets = useWidgets(control);
  const ruleTemplates = useRuleTemplates(control);
  const ruleProfiles = useRuleProfiles(control);
  const globals = useGlobals(control);
  const plugins = usePlugins(control, {
    translate,
    refreshBehavior: () => automation.refresh(),
    reportError: (message) => {
      automation.behaviorError.value = message;
    },
    pickPluginPackage: (onSelected) => {
      media.openMediaPicker(
        pluginPickerOptions(translate('pluginInstallPickerTitle')),
        (selection, pickerError) => {
          if (pickerError) {
            onSelected(null, pickerError);
            return;
          }
          if (!selection || selection.type !== 'file') {
            onSelected(null);
            return;
          }
          onSelected(selection.file.path);
        },
      );
    },
  });
  const processors = useProcessors(control);
  const media = useMedia(control);
  // Speech is plugin-owned now: the TTS plugin backend subscribes to chat
  // events and speaks them. The main frontend renders chat only.
  const live = useLive(control, {
    systemAuthor: () => translate('system'),
  });

  // Reliable-gap resync: the host skipped authoritative events this client
  // never saw, so every authoritative snapshot is re-read (live status,
  // points config/leaderboard, creators, automation/workflows including
  // plugin state, gifts, globals, profiles). Missing events are never reconstructed.
  control.onGap(() => {
    void connection.refreshStatus();
    void points.refresh();
    void creators.refresh();
    void automation.refresh();
    void live.refresh();
    void globals.loadGlobals();
    void ruleProfiles.loadProfiles();
  });

  watch(locale, (value) => {
    document.documentElement.lang = value;
    saveLocale(value);
  });

  watch(theme, (value) => {
    applyTheme(value);
    saveTheme(value);
  });

  watch([live.events, live.autoScroll, activeTab], () => {
    if (activeTab.value !== 'feed' || !live.autoScroll.value) return;
    requestAnimationFrame(() => live.scrollToBottom());
  });

  // A plugin page tab is only valid while its plugin stays installed,
  // enabled, and available; the host drops its pages from the snapshot
  // otherwise, and the UI falls back to the plugins list.
  watch([automation.behavior, activeTab], () => {
    const parsed = parsePluginNavId(activeTab.value);
    if (!parsed) return;
    // Annotated locals keep the generic-inference chain shallow for vue-tsc.
    const pages: PluginPageDescriptor[] = automation.pluginPages.value;
    const exists = pages.some(
      (page) => page.pluginId === parsed.pluginId && page.id === parsed.pageId,
    );
    if (!exists) activeTab.value = 'plugins';
  });

  onMounted(() => {
    control.attach();
    // Same initial state the legacy mount sequence fetched, now as
    // JSON-RPC reads. Each refresh reports its own failures.
    void points.refresh();
    void creators.refresh();
    void automation.refresh();
    void live.refresh();
    void processors.refresh();
    void globals.loadGlobals();
    void ruleProfiles.loadProfiles();

    // Keep the saved username in the connect form, but wait for an explicit
    // user action before starting network work on a cold launch.
  });

  onUnmounted(() => {
    control.detach();
  });

  const setActiveTab = (value: AppTab): void => {
    activeTab.value = value;
  };
  const setLocale = (value: Locale): void => {
    locale.value = value;
  };
  const setTheme = (value: Theme): void => {
    theme.value = value;
  };
  const handleThemeToggle = (): void => {
    theme.value = theme.value === 'dark' ? 'light' : 'dark';
  };
  const handleLocaleToggle = (): void => {
    locale.value = locale.value === 'en' ? 'es' : 'en';
  };
  const openPlugins = (): void => {
    activeTab.value = 'plugins';
  };

  /**
   * Applies a profile: creates every entry's records, registers the pack
   * membership, marks it active, then refreshes once. Failures surface on
   * the profiles error channel; the modal stays open to show them.
   */
  const handleApplyRuleProfile = async (
    profile: RuleProfile,
    applied: AppliedRuleTemplate[],
  ): Promise<void> => {
    try {
      await automation.createProfileRecords(applied);
      await ruleProfiles.registerPack(profile, applied);
      await automation.refresh();
    } catch (failure) {
      ruleProfiles.setError(errorMessage(failure));
      throw failure;
    }
  };

  const handleSwitchRuleProfile = async (id: string): Promise<void> => {
    try {
      await ruleProfiles.switchProfile(id, {
        eventIds: automation.behavior.value.events.map((event) => event.id),
        actionIds: automation.behavior.value.actions.map((action) => action.id),
      });
      await automation.refresh();
    } catch (failure) {
      ruleProfiles.setError(errorMessage(failure));
      throw failure;
    }
  };

  const handleCreateRuleProfile = async (name: string): Promise<void> => {
    try {
      await ruleProfiles.createPack(name);
    } catch (failure) {
      ruleProfiles.setError(errorMessage(failure));
      throw failure;
    }
  };

  const handleDeleteRuleProfile = async (id: string): Promise<void> => {
    try {
      await ruleProfiles.deletePack(id);
      await automation.refresh();
    } catch (failure) {
      ruleProfiles.setError(errorMessage(failure));
      throw failure;
    }
  };

  const handleExportRuleProfile = (id: string): void => {
    const pack = resolvedPacks(
      ruleProfiles.packs.value,
      automation.behavior.value.events.map((event) => event.id),
      automation.behavior.value.actions.map((action) => action.id),
    ).find((entry) => entry.id === id);
    if (!pack) {
      ruleProfiles.setError(`unknown profile ${id}`);
      return;
    }
    const exported = buildProfileDoc(pack, automation.behavior.value.events, automation.behavior.value.actions);
    downloadTextFile(`${pack.id}.tikprofile.json`, `${JSON.stringify(exported.doc, null, 2)}\n`);
  };

  /** Saves through automation, then adopts the rule into the active pack. */
  const handleSaveBehaviorAction: typeof automation.handleSaveAction = (action) => {
    void (async (): Promise<void> => {
      try {
        await automation.saveRecordAndRefresh('action', action);
      } catch {
        return;
      }
      await ruleProfiles.adoptRule('action', action.id);
    })();
  };

  /** Saves through automation, then adopts the rule into the active pack. */
  const handleSaveBehaviorEvent: typeof automation.handleSaveEvent = (event) => {
    void (async (): Promise<void> => {
      try {
        await automation.saveRecordAndRefresh('event', event);
      } catch {
        return;
      }
      await ruleProfiles.adoptRule('event', event.id);
    })();
  };

  // Grouped services (incremental direction: new call sites consume
  // these namespaces; the flat fields below stay for existing call sites).
  const navigation = {
    activeTab,
    setActiveTab,
    openPlugins,
  };
  const settings = {
    locale,
    theme,
    setLocale,
    setTheme,
    handleThemeToggle,
    handleLocaleToggle,
  };
  // Host side of one inline plugin frame: a broker backend scoped to the
  // plugin id plus topic subscriptions filtered to that plugin's own
  // events. Cheap closures — subscriptions start when the frame mounts.
  const createPluginBackend = (
    pluginId: string,
  ): {
    backend: BrokerBackend;
    subscribeTopic: (topic: string, listener: (data: unknown) => void) => () => void;
  } => ({
    backend: controlBackend(pluginId, control, () => locale.value, () => theme.value),
    subscribeTopic: (topic, listener) =>
      control.onTopic(topic, (data: unknown) => {
        if (
          data !== null &&
          typeof data === 'object' &&
          (data as Record<string, unknown>)['pluginId'] === pluginId
        ) {
          listener(data);
        }
      }),
  });
  const pluginUi = {
    pluginPages: automation.pluginPages,
    pluginUis: automation.pluginUis,
    createPluginBackend,
    pluginSettings: plugins.pluginSettings,
    actionOptions: plugins.actionOptions,
    actionOptionErrors: plugins.actionOptionErrors,
    actionOptionSelected: plugins.actionOptionSelected,
    pluginConnections: plugins.pluginConnections,
    pluginProvision: plugins.pluginProvision,
    handleGetPluginSettings: plugins.handleGetPluginSettings,
    handleSavePluginSettings: plugins.handleSavePluginSettings,
    handleGetActionOptions: plugins.handleGetActionOptions,
    handleTestPluginConnection: plugins.handleTestPluginConnection,
    handleProvisionPluginToken: plugins.handleProvisionPluginToken,
    executePluginAction: plugins.executeAction,
    openMediaPicker: media.openMediaPicker,
  };
  return {
    navigation,
    settings,
    pluginUi,
    live,
    points,
    automation,
    plugins,
    activeTab,
    uniqueId: connection.uniqueId,
    cookie: connection.cookie,
    activeCreator: connection.activeCreator,
    recents: connection.recents,
    locale,
    theme,
    status: connection.status,
    error: connection.error,
    events: live.events,
    filter: live.filter,
    searchQuery: live.searchQuery,
    pointsConfig: points.pointsConfig,
    leaderboard: points.leaderboard,
    topViewers: live.topViewers,
    liveViewers: live.liveViewers,
    activeCreatorRecord: creators.activeCreatorRecord,
    recentCreators: creators.recentCreators,
    behavior: automation.behavior,
    giftCatalog: live.giftCatalog,
    behaviorRuns: automation.behaviorRuns,
    behaviorTestRuns: automation.behaviorTestRuns,
    behaviorError: automation.behaviorError,
    hotkeyStatus: automation.hotkeyStatus,
    lastHotkeyEvent: automation.lastHotkeyEvent,
    pluginSettings: plugins.pluginSettings,
    processors: processors.processors,
    processorTest: processors.processorTest,
    actionOptions: plugins.actionOptions,
    actionOptionErrors: plugins.actionOptionErrors,
    pluginConnections: plugins.pluginConnections,
    pluginPages: automation.pluginPages,
    pluginUis: automation.pluginUis,
    createPluginBackend,
    pluginProgress: plugins.pluginProgress,
    dismissPluginProgress: plugins.dismissPluginProgress,
    autoScroll: live.autoScroll,
    unreadCount: live.unreadCount,
    resetEvents: live.resetEvents,
    setActiveTab,
    setUniqueId: connection.setUniqueId,
    setCookie: connection.setCookie,
    setLocale,
    setTheme,
    setFilter: live.setFilter,
    setSearchQuery: live.setSearchQuery,
    setStreamContainerRef: live.setStreamContainerRef,
    openPlugins,
    handleConnect: connection.handleConnect,
    handlePickLive: connection.handlePickLive,
    handleDisconnect: connection.handleDisconnect,
    handleReconnect: connection.handleReconnect,
    handleSelectRecent: connection.handleSelectRecent,
    handleToggleAutoScroll: live.handleToggleAutoScroll,
    handleThemeToggle,
    handleLocaleToggle,
    handleUpdatePointsConfig: points.handleUpdatePointsConfig,
    handleResetPoints: points.handleResetPoints,
    handleAdjustPoints: points.handleAdjustPoints,
    handleSaveAction: handleSaveBehaviorAction,
    handleApplyRuleTemplate: automation.handleApplyRuleTemplate,
    ruleTemplateCustom: ruleTemplates.custom,
    ruleTemplateError: ruleTemplates.error,
    loadRuleTemplateCustom: ruleTemplates.loadCustom,
    importRuleTemplates: ruleTemplates.importTemplates,
    deleteRuleTemplateCustom: ruleTemplates.deleteCustom,
    ruleProfilePacks: ruleProfiles.packs,
    activeRuleProfileId: ruleProfiles.activeId,
    ruleProfileError: ruleProfiles.error,
    loadRuleProfiles: ruleProfiles.loadProfiles,
    handleApplyRuleProfile,
    handleSwitchRuleProfile,
    handleCreateRuleProfile,
    handleDeleteRuleProfile,
    handleExportRuleProfile,
    globals: globals.globals,
    globalsLoading: globals.loading,
    globalsError: globals.error,
    loadGlobals: globals.loadGlobals,
    saveGlobals: globals.saveGlobals,
    handleDeleteAction: automation.handleDeleteAction,
    handleSetActionEnabled: automation.handleSetActionEnabled,
    handleTestAction: automation.handleTestAction,
    handleSaveEvent: handleSaveBehaviorEvent,
    handleDeleteEvent: automation.handleDeleteEvent,
    handleSetEventEnabled: automation.handleSetEventEnabled,
    handleTestEvent: automation.handleTestEvent,
    handleFireEvent: automation.handleFireEvent,
    hotkeyAccessPending: automation.hotkeyAccessPending,
    handleRequestHotkeyAccess: automation.handleRequestHotkeyAccess,
    handleSetPluginInstalled: plugins.handleSetPluginInstalled,
    handleUninstallPlugin: plugins.handleUninstallPlugin,
    handleSetPluginEnabled: plugins.handleSetPluginEnabled,
    handleGetPluginSettings: plugins.handleGetPluginSettings,
    handleSavePluginSettings: plugins.handleSavePluginSettings,
    handleGetProcessorStatus: processors.handleGetProcessorStatus,
    handleTestProcessor: processors.handleTestProcessor,
    handleGetActionOptions: plugins.handleGetActionOptions,
    handleTestPluginConnection: plugins.handleTestPluginConnection,
    pluginProvision: plugins.pluginProvision,
    handleProvisionPluginToken: plugins.handleProvisionPluginToken,
    executePluginAction: plugins.executeAction,
    actionOptionSelected: plugins.actionOptionSelected,
    analyticsSummary: analytics.analyticsSummary,
    handleGetAnalyticsRange: analytics.handleGetAnalyticsRange,
    pluginInstallState: plugins.pluginInstallState,
    handleInstallPlugin: plugins.handleInstallPlugin,
    handleConfirmPluginReplace: plugins.handleConfirmPluginReplace,
    handleCancelPluginReplace: plugins.handleCancelPluginReplace,
    openMediaPicker: media.openMediaPicker,
    handleAnalyzeScript: automation.handleAnalyzeScript,
    widgetsStatus: widgets.status,
    widgetDesigns: widgets.designs,
    widgetDesignsLoading: widgets.designsLoading,
    widgetDesignsError: widgets.designsError,
    loadWidgetDesigns: widgets.loadDesigns,
    saveWidgetDesign: widgets.saveDesign,
    widgetsStatusError: widgets.statusError,
    widgetsRefreshing: widgets.refreshing,
    refreshWidgetsStatus: widgets.refreshStatus,
    copyWidgetObsUrl: widgets.copyObsUrl,
  };
}
