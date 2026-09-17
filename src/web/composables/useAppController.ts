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
import { useTts } from '../features/tts.ts';
import { createControlClient } from '../platform/control-client.ts';
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
  const tts = useTts(control, plugins.actionOptions, automation.pluginPages, {
    executeAction: (actionType, config, live) =>
      plugins.executeAction(actionType, config, live),
    refreshOptions: (source) => plugins.handleGetActionOptions(source),
    adjustPoints: (uniqueId, delta) => points.handleAdjustPoints(uniqueId, delta),
    leaderboardPointsFor: (handle) => points.leaderboardPointsFor(handle),
  });
  const live = useLive(control, {
    onChat: (author, text, pointsValue, isSubscriber) =>
      tts.runAutoTts(author, text, pointsValue, isSubscriber),
    systemAuthor: () => translate('system'),
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
    void tts.refresh();

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

  return {
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
    pluginSettings: plugins.pluginSettings,
    processors: processors.processors,
    processorTest: processors.processorTest,
    actionOptions: plugins.actionOptions,
    actionOptionErrors: plugins.actionOptionErrors,
    pluginConnections: plugins.pluginConnections,
    pluginPages: automation.pluginPages,
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
    handleSaveAction: automation.handleSaveAction,
    handleDeleteAction: automation.handleDeleteAction,
    handleSetActionEnabled: automation.handleSetActionEnabled,
    handleTestAction: automation.handleTestAction,
    handleSaveEvent: automation.handleSaveEvent,
    handleDeleteEvent: automation.handleDeleteEvent,
    handleSetEventEnabled: automation.handleSetEventEnabled,
    handleTestEvent: automation.handleTestEvent,
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
    ttsSettings: tts.ttsSettings,
    ttsSpeaking: tts.ttsSpeaking,
    ttsLogs: tts.ttsLogs,
    ttsSettingsOrDefault: tts.ttsSettingsOrDefault,
    handleTtsSettingsChange: tts.handleTtsSettingsChange,
    handleTtsSpeak: tts.handleTtsSpeak,
    actionOptionSelected: plugins.actionOptionSelected,
    ttsOutputPending: tts.ttsOutputPending,
    ttsOutputErrors: tts.ttsOutputErrors,
    handleTtsOutputSelect: tts.handleTtsOutputSelect,
    analyticsSummary: analytics.analyticsSummary,
    handleGetAnalyticsRange: analytics.handleGetAnalyticsRange,
    pluginInstallState: plugins.pluginInstallState,
    handleInstallPlugin: plugins.handleInstallPlugin,
    handleConfirmPluginReplace: plugins.handleConfirmPluginReplace,
    handleCancelPluginReplace: plugins.handleCancelPluginReplace,
    openMediaPicker: media.openMediaPicker,
    handleAnalyzeScript: automation.handleAnalyzeScript,
  };
}
