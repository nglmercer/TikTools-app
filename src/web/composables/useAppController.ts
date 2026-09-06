import { onMounted, onUnmounted, ref, watch } from 'vue';

import type {
  ActionOptionItem,
  GiftCatalogEntry,
  HostMessage,
  HotkeyStatusData,
  MediaPickerOptions,
  MediaSelectionHandler,
  PageMessage,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { AutomationEventType } from '../../automation/types.ts';
import type { BehaviorRun, BehaviorSnapshot, LiveAction, LiveEvent } from '../../automation/behavior/types.ts';
import {
  addRecentUsername,
  applyTheme,
  getInitialLocale,
  getInitialTheme,
  getRecentUsernames,
  getSavedUsername,
  saveLocale,
  saveTheme,
  saveUsername,
  type Theme,
} from '../preferences.ts';
import { setPluginTranslations, t, type Locale } from '../i18n.ts';
import { setPluginEventTypes } from '../../automation/event-registry.ts';
import type {
  AppTab,
  ConnectionStatus,
  CreatorRecord,
  DisplayEvent,
  EventFilter,
  PluginSettingsState,
  PointsConfig,
  StreamTelemetry,
  TopViewerPayload,
  ViewerRecord,
} from '../types.ts';

declare global {
  interface Window {
    ipc?: { postMessage: (message: string) => void };
    __webview_on_message__?: (message: string) => void;
    __tiktools_host_message_queue__?: string[];
  }
}

const initialLocale = getInitialLocale();
const initialTheme = getInitialTheme();
const initialUsername = getSavedUsername();

export const defaultPointsConfig: PointsConfig = {
  currencyName: 'Points',
  pointsPerCoin: 1.0,
  pointsPerCoinEnabled: true,
  pointsPerShare: 3.0,
  pointsPerShareEnabled: true,
  pointsPerChat: 1.0,
  pointsPerChatEnabled: true,
  pointsPerLike: 0.1,
  pointsPerLikeEnabled: true,
  pointsPerFollow: 5.0,
  pointsPerFollowEnabled: true,
  pointsPerJoin: 0.5,
  pointsPerJoinEnabled: false,
  subBonusMultiplier: 0.0,
  pointsPerLevel: 100,
};

function send(message: PageMessage): void {
  window.ipc?.postMessage(JSON.stringify(message));
}

function normalizeUsername(value: string): string {
  return value.trim().replace(/^@/, '');
}

import {
  applyPluginInstallResult,
  createInitialPluginInstallState,
  installPackageMessage,
  pluginPickerOptions,
  type PluginInstallState,
} from './plugin-install.ts';

export type { PluginInstallState };

export const initialPluginInstallState = createInitialPluginInstallState();

export function useAppController() {
  const activeTab = ref<AppTab>('feed');
  const uniqueId = ref(initialUsername);
  const cookie = ref('');
  const activeCreator = ref(initialUsername);
  const recents = ref<string[]>(getRecentUsernames());

  const locale = ref<Locale>(initialLocale);
  const theme = ref<Theme>(initialTheme);
  const status = ref<ConnectionStatus>('idle');
  const error = ref('');

  const events = ref<DisplayEvent[]>([]);
  const filter = ref<EventFilter>('all');
  const searchQuery = ref('');

  const pointsConfig = ref<PointsConfig>(defaultPointsConfig);
  const leaderboard = ref<ViewerRecord[]>([]);
  const topViewers = ref<TopViewerPayload[]>([]);
  const liveViewers = ref(0);
  const activeCreatorRecord = ref<CreatorRecord | null>(null);
  const recentCreators = ref<CreatorRecord[]>([]);

  const behavior = ref<BehaviorSnapshot>({ actions: [], events: [], plugins: [], actionTypes: [], translations: {} });
  const giftCatalog = ref<GiftCatalogEntry[]>([]);
  const behaviorRuns = ref<BehaviorRun[]>([]);
  const behaviorTestRuns = ref<BehaviorRun[]>([]);
  const behaviorError = ref('');
  const hotkeyStatus = ref<HotkeyStatusData | null>(null);
  const pluginSettings = ref<Record<string, PluginSettingsState>>({});
  const actionOptions = ref<Record<string, ActionOptionItem[]>>({});
  const pluginInstallState = ref<PluginInstallState>({ ...initialPluginInstallState });
  const pluginProgress = ref<Extract<HostMessage, { type: 'plugin-progress' }> | null>(null);
  const mediaSelectionHandlers = new Map<string, MediaSelectionHandler>();
  let mediaRequestSequence = 0;
  let pluginProgressTimer: ReturnType<typeof setTimeout> | undefined;

  const telemetry = ref<StreamTelemetry>({ chats: 0, gifts: 0, likes: 0, members: 0 });
  const autoScroll = ref(true);
  const unreadCount = ref(0);

  const nextEventId = ref(0);
  const activeCreatorRef = ref(initialUsername);
  const streamContainerRef = ref<HTMLDivElement | null>(null);

  applyTheme(initialTheme);
  document.documentElement.lang = initialLocale;

  const resetEvents = (): void => {
    nextEventId.value = 0;
    events.value = [];
    unreadCount.value = 0;
    telemetry.value = { chats: 0, gifts: 0, likes: 0, members: 0 };
    topViewers.value = [];
    liveViewers.value = 0;
  };

  watch(locale, (value) => {
    document.documentElement.lang = value;
    saveLocale(value);
  });

  watch(theme, (value) => {
    applyTheme(value);
    saveTheme(value);
  });

  watch([events, autoScroll, activeTab], () => {
    if (activeTab.value !== 'feed' || !autoScroll.value || !streamContainerRef.value) return;
    requestAnimationFrame(() => {
      const container = streamContainerRef.value;
      if (container) container.scrollTop = container.scrollHeight;
    });
  });

  const receive = (raw: string): void => {
    let message: HostMessage;
    try {
      message = JSON.parse(raw) as HostMessage;
    } catch {
      return;
    }

    if (message.type === 'connection') {
      if (message.status === 'connecting') status.value = 'connecting';
      if (message.status === 'connected') {
        status.value = 'connected';
        if (message.uniqueId) {
          const clean = normalizeUsername(message.uniqueId);
          activeCreator.value = clean;
          activeCreatorRef.value = clean;
          recents.value = addRecentUsername(clean);
        }
      }
      if (message.status === 'disconnected') status.value = 'disconnected';
    }

    if (message.type === 'reconnecting') status.value = 'retrying';

    if (message.type === 'room-stats') {
      topViewers.value = message.topViewers;
      liveViewers.value = message.viewers;
    }

    if (message.type === 'points-config') pointsConfig.value = message.config;
    if (message.type === 'leaderboard') leaderboard.value = message.viewers;

    if (message.type === 'points-awarded') {
      const index = leaderboard.value.findIndex((viewer) => viewer.uniqueId === message.uniqueId);
      if (index >= 0) {
        const updated = [...leaderboard.value];
        const current = updated[index];
        if (current) {
          updated[index] = {
            ...current,
            points: message.totalPoints,
            level: message.level,
            lastSeen: Date.now(),
          };
        }
        leaderboard.value = updated.sort((left, right) => right.points - left.points);
      }
    }

    if (message.type === 'live-event') {
      const event = message.event;
      telemetry.value = {
        chats: telemetry.value.chats + (event.kind === 'chat' ? 1 : 0),
        gifts: telemetry.value.gifts + (event.kind === 'gift' ? 1 : 0),
        likes: telemetry.value.likes + (event.kind === 'like' ? 1 : 0),
        members: telemetry.value.members + (event.kind === 'member' || event.kind === 'social' ? 1 : 0),
      };
      events.value = [
        ...events.value,
        { ...event, id: nextEventId.value++, receivedAt: Date.now() },
      ].slice(-300);
      if (!autoScroll.value) unreadCount.value += 1;
    }

    if (message.type === 'error') {
      status.value = 'error';
      error.value = message.message;
      events.value = [
        ...events.value,
        {
          kind: 'member' as const,
          author: t(locale.value, 'system'),
          text: message.message,
          id: nextEventId.value++,
          receivedAt: Date.now(),
        },
      ].slice(-300);
    }

    if (message.type === 'media-selected') {
      const handler = mediaSelectionHandlers.get(message.requestId);
      if (handler) {
        mediaSelectionHandlers.delete(message.requestId);
        handler(message.selection ?? null, message.error);
      }
    }

    if (message.type === 'creator-state') {
      activeCreatorRecord.value = message.creator;
      if (message.creator?.uniqueId) {
        const clean = normalizeUsername(message.creator.uniqueId);
        activeCreator.value = clean;
        activeCreatorRef.value = clean;
        recents.value = addRecentUsername(clean);
        saveUsername(clean);
      }
    }

    if (message.type === 'recent-creators') {
      recentCreators.value = message.creators;
      const names = message.creators.map((creator) => creator.uniqueId);
      if (names.length > 0) {
        recents.value = [...new Set([...names, ...recents.value])].slice(0, 10);
      }
    }

    if (message.type === 'app-state') console.log('[app-state]', message.state);

    if (message.type === 'gift-catalog') {
      giftCatalog.value = message.gifts;
      return;
    }

    if (message.type === 'behavior') {
      setPluginTranslations(message.snapshot.translations);
      setPluginEventTypes(message.snapshot.eventTypes ?? []);
      behavior.value = message.snapshot;
      if (!message.snapshot.plugins.some((plugin) => plugin.descriptor.id === 'hotkeys' && plugin.installed && plugin.enabled)) {
        hotkeyStatus.value = null;
      }
      behaviorError.value = '';
    }
    if (message.type === 'hotkey-status') hotkeyStatus.value = message.status;
    if (message.type === 'behavior-runs') behaviorRuns.value = message.runs;
    if (message.type === 'behavior-test-result') behaviorTestRuns.value = message.runs;
    if (message.type === 'behavior-error') behaviorError.value = message.message;
    if (message.type === 'automation-error') behaviorError.value = message.message;

    if (message.type === 'plugin-settings') {
      pluginSettings.value = {
        ...pluginSettings.value,
        [message.id]: { schema: message.schema, uiHints: message.uiHints, values: message.values },
      };
      behaviorError.value = '';
    }

    if (message.type === 'plugin-progress') {
      pluginProgress.value = message;
      if (pluginProgressTimer) clearTimeout(pluginProgressTimer);
      if (message.state === 'ready' || message.state === 'failed') {
        pluginProgressTimer = setTimeout(() => {
          pluginProgress.value = null;
          pluginProgressTimer = undefined;
        }, message.state === 'failed' ? 10_000 : 4_000);
      }
    }

    if (message.type === 'action-options') {
      actionOptions.value = { ...actionOptions.value, [message.source]: message.options };
    }

    if (message.type === 'plugin-install-result') {
      // The behavior snapshot emitted by the backend refreshes the list.
      pluginInstallState.value = applyPluginInstallResult(
        pluginInstallState.value,
        message,
        (key) => t(locale.value, key),
      );
    }

    if (message.type === 'plugin-uninstall-result' && !message.success) {
      behaviorError.value = message.error;
    }

    if (message.type === 'gift-debug') {
      console.warn(
        `[gift-debug] giftId=${message.giftId} hasIcon=${message.hasIcon} totalGifts=${message.totalGifts} icon=${message.iconUrl?.slice(0, 80) || 'MISSING'}`,
      );
      if (!message.hasIcon) {
        console.warn('[gift-debug] gift has no icon; giftList may not contain this giftId for this room', message.totalGifts);
      }
    }
  };

  onMounted(() => {
    window.__webview_on_message__ = receive;
    const pending = window.__tiktools_host_message_queue__ ?? [];
    window.__tiktools_host_message_queue__ = [];
    pending.forEach(receive);

    send({ type: 'get-points-config' });
    send({ type: 'get-leaderboard', limit: 100 });
    send({ type: 'get-creator' });
    send({ type: 'get-recent-creators', limit: 10 });
    send({ type: 'get-app-state' });
    send({ type: 'get-behavior' });
    send({ type: 'get-gift-catalog' });

    // Keep the saved username in the connect form, but wait for an explicit
    // user action before starting network work on a cold launch.
  });

  onUnmounted(() => {
    if (window.__webview_on_message__ === receive) window.__webview_on_message__ = undefined;
    mediaSelectionHandlers.clear();
    if (pluginProgressTimer) clearTimeout(pluginProgressTimer);
  });

  const handleConnect = (userToConnect?: string): void => {
    const target = normalizeUsername(userToConnect || uniqueId.value);
    if (!target) {
      error.value = t(locale.value, 'handleRequired');
      return;
    }
    error.value = '';
    resetEvents();
    status.value = 'connecting';
    activeCreator.value = target;
    activeCreatorRef.value = target;
    saveUsername(target);
    recents.value = addRecentUsername(target);
    activeTab.value = 'feed';
    send({ type: 'connect', uniqueId: target, sessionCookie: cookie.value.trim() });
  };

  const handlePickLive = (): void => {
    error.value = '';
    resetEvents();
    status.value = 'connecting';
    activeCreator.value = t(locale.value, 'searchingRooms');
    activeTab.value = 'feed';
    send({ type: 'pick-live', sessionCookie: cookie.value.trim() });
  };

  const handleDisconnect = (): void => {
    send({ type: 'disconnect' });
    status.value = 'disconnected';
  };

  const handleReconnect = (): void => {
    if (activeCreatorRef.value) handleConnect(activeCreatorRef.value);
  };

  const handleSelectRecent = (username: string): void => {
    uniqueId.value = username;
    handleConnect(username);
  };

  const handleToggleAutoScroll = (): void => {
    const nextState = !autoScroll.value;
    autoScroll.value = nextState;
    if (nextState) {
      unreadCount.value = 0;
      const container = streamContainerRef.value;
      if (container) container.scrollTop = container.scrollHeight;
    }
  };

  const handleThemeToggle = (): void => {
    theme.value = theme.value === 'dark' ? 'light' : 'dark';
  };

  const handleLocaleToggle = (): void => {
    locale.value = locale.value === 'en' ? 'es' : 'en';
  };

  const clearBehaviorError = (): void => { behaviorError.value = ''; };
  const handleUpdatePointsConfig = (config: Partial<PointsConfig>): void => send({ type: 'update-points-config', config });
  const handleResetPoints = (uniqueId?: string): void => send({ type: 'reset-points', uniqueId });
  const handleAdjustPoints = (uniqueId: string, delta: number): void => send({ type: 'adjust-points', uniqueId, delta });

  const handleSaveAction = (action: LiveAction): void => { clearBehaviorError(); send({ type: 'save-action', action }); };
  const handleDeleteAction = (id: string): void => { clearBehaviorError(); send({ type: 'delete-action', id }); };
  const handleSetActionEnabled = (id: string, enabled: boolean): void => { clearBehaviorError(); send({ type: 'set-action-enabled', id, enabled }); };
  const handleTestAction = (action: LiveAction, trigger?: string): void => {
    clearBehaviorError();
    behaviorTestRuns.value = [];
    send({ type: 'test-action', action, trigger });
  };
  const handleSaveEvent = (event: LiveEvent): void => { clearBehaviorError(); send({ type: 'save-event', event }); };
  const handleDeleteEvent = (id: string): void => { clearBehaviorError(); send({ type: 'delete-event', id }); };
  const handleSetEventEnabled = (id: string, enabled: boolean): void => { clearBehaviorError(); send({ type: 'set-event-enabled', id, enabled }); };
  const handleTestEvent = (event: LiveEvent): void => {
    clearBehaviorError();
    behaviorTestRuns.value = [];
    send({ type: 'test-event', event });
  };
  const handleSetPluginInstalled = (id: string, installed: boolean): void => { clearBehaviorError(); send({ type: 'set-plugin-install', id, installed }); };
  const handleUninstallPlugin = (id: string): void => { clearBehaviorError(); send({ type: 'uninstall-plugin-package', id }); };
  const handleSetPluginEnabled = (id: string, enabled: boolean): void => { clearBehaviorError(); send({ type: 'set-plugin-enabled', id, enabled }); };
  const handleGetPluginSettings = (id: string): void => send({ type: 'get-plugin-settings', id });
  const handleSavePluginSettings = (id: string, values: PluginSettingValues): void => { clearBehaviorError(); send({ type: 'save-plugin-settings', id, values }); };
  const handleGetActionOptions = (source: string): void => send({ type: 'get-action-options', source });

  const openMediaPicker = (options: MediaPickerOptions, onSelected: MediaSelectionHandler): void => {
    const requestId = `media-${Date.now()}-${++mediaRequestSequence}`;
    mediaSelectionHandlers.set(requestId, onSelected);
    if (!window.ipc) {
      mediaSelectionHandlers.delete(requestId);
      onSelected(null, 'Native media picker is unavailable in this preview.');
      return;
    }
    const message: PageMessage = {
      type: 'open-media-picker',
      requestId,
      mode: options.mode ?? 'file',
      kind: options.kind ?? 'audio',
      ...(options.title ? { title: options.title } : {}),
      ...(options.initialDirectory ? { initialDirectory: options.initialDirectory } : {}),
      ...(options.extensions?.length ? { extensions: options.extensions.slice(0, 32) } : {}),
    };
    send(message);
  };

  const sendInstallPackage = (path: string, replaceExisting: boolean): void => {
    pluginInstallState.value = {
      installing: true,
      error: '',
      success: '',
      pendingPath: path,
      needsReplace: false,
    };
    send(installPackageMessage(path, replaceExisting));
  };

  const handleInstallPlugin = (): void => {
    if (pluginInstallState.value.installing) return;
    pluginInstallState.value = createInitialPluginInstallState();
    openMediaPicker(
      pluginPickerOptions(t(locale.value, 'pluginInstallPickerTitle')),
      (selection, pickerError) => {
        if (pickerError) {
          pluginInstallState.value = {
            installing: false,
            error: pickerError,
            success: '',
            pendingPath: '',
            needsReplace: false,
          };
          return;
        }
        // Picker cancellation sends nothing.
        if (!selection || selection.type !== 'file') return;
        sendInstallPackage(selection.file.path, false);
      },
    );
  };

  const handleConfirmPluginReplace = (): void => {
    const pending = pluginInstallState.value.pendingPath;
    if (!pending || pluginInstallState.value.installing) return;
    sendInstallPackage(pending, true);
  };

  const handleCancelPluginReplace = (): void => {
    pluginInstallState.value = createInitialPluginInstallState();
  };

  const setActiveTab = (value: AppTab): void => { activeTab.value = value; };
  const setUniqueId = (value: string): void => { uniqueId.value = value; };
  const setCookie = (value: string): void => { cookie.value = value; };
  const setLocale = (value: Locale): void => { locale.value = value; };
  const setTheme = (value: Theme): void => { theme.value = value; };
  const setFilter = (value: EventFilter): void => { filter.value = value; };
  const setSearchQuery = (value: string): void => { searchQuery.value = value; };
  const setStreamContainerRef = (element: Element | null): void => {
    streamContainerRef.value = element instanceof HTMLDivElement ? element : null;
  };
  const openPlugins = (): void => { activeTab.value = 'plugins'; };
  const dismissPluginProgress = (): void => {
    pluginProgress.value = null;
    if (pluginProgressTimer) {
      clearTimeout(pluginProgressTimer);
      pluginProgressTimer = undefined;
    }
  };

  return {
    activeTab,
    uniqueId,
    cookie,
    activeCreator,
    recents,
    locale,
    theme,
    status,
    error,
    events,
    filter,
    searchQuery,
    pointsConfig,
    leaderboard,
    topViewers,
    liveViewers,
    activeCreatorRecord,
    recentCreators,
    behavior,
    giftCatalog,
    behaviorRuns,
    behaviorTestRuns,
    behaviorError,
    hotkeyStatus,
    pluginSettings,
    actionOptions,
    pluginProgress,
    dismissPluginProgress,
    telemetry,
    autoScroll,
    unreadCount,
    resetEvents,
    setActiveTab,
    setUniqueId,
    setCookie,
    setLocale,
    setTheme,
    setFilter,
    setSearchQuery,
    setStreamContainerRef,
    openPlugins,
    handleConnect,
    handlePickLive,
    handleDisconnect,
    handleReconnect,
    handleSelectRecent,
    handleToggleAutoScroll,
    handleThemeToggle,
    handleLocaleToggle,
    handleUpdatePointsConfig,
    handleResetPoints,
    handleAdjustPoints,
    handleSaveAction,
    handleDeleteAction,
    handleSetActionEnabled,
    handleTestAction,
    handleSaveEvent,
    handleDeleteEvent,
    handleSetEventEnabled,
    handleTestEvent,
    handleSetPluginInstalled,
    handleUninstallPlugin,
    handleSetPluginEnabled,
    handleGetPluginSettings,
    handleSavePluginSettings,
    handleGetActionOptions,
    pluginInstallState,
    handleInstallPlugin,
    handleConfirmPluginReplace,
    handleCancelPluginReplace,
    openMediaPicker,
    handleAnalyzeScript: (nodeId: string, source: string, offset: number, eventType?: AutomationEventType): void => {
      send({ type: 'analyze-automation-script', nodeId, source, offset, eventType });
    },
  };
}
