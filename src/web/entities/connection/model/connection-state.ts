import { ref } from 'vue';

import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import {
  addRecentUsername,
  getRecentUsernames,
  getSavedUsername,
  saveUsername,
} from '../lib/creator-preferences.ts';
import type { ConnectionStatus } from '@/types.ts';

export interface LiveStatusResult {
  connected: boolean;
  uniqueId: string | null;
  roomId: string | null;
  connectionId: string | null;
  native: boolean;
}

export interface ConnectionCallbacks {
  goFeed: () => void;
  resetFeed: () => void;
  translate: (key: string) => string;
}

function normalizeUsername(value: string): string {
  return value.trim().replace(/^@/, '');
}

const MAX_RECONNECT_ATTEMPTS = 5;

export function useConnection(control: ControlClient, callbacks: ConnectionCallbacks) {
  const uniqueId = ref(getSavedUsername());
  const cookie = ref('');
  const activeCreator = ref(getSavedUsername());
  const activeCreatorRef = ref(getSavedUsername());
  const recents = ref<string[]>(getRecentUsernames());
  const status = ref<ConnectionStatus>('idle');
  const error = ref('');
  const reconnectAttempt = ref<number | null>(null);
  const reconnectDelayMs = ref<number | null>(null);
  let revision = 0;
  let statusRefresh = 0;
  const unsubscribers: Array<() => void> = [];
  const subscribe: ControlClient['onTopic'] = (topic, handler) => {
    const unsubscribe = control.onTopic(topic, handler);
    unsubscribers.push(unsubscribe);
    return unsubscribe;
  };

  const noteCreatorSeen = (clean: string, persist: boolean): void => {
    activeCreator.value = clean;
    activeCreatorRef.value = clean;
    uniqueId.value = clean;
    recents.value = addRecentUsername(clean);
    if (persist) saveUsername(clean);
  };

  const mergeRecentNames = (names: string[]): void => {
    if (names.length > 0) {
      recents.value = [...new Set([...names, ...recents.value])].slice(0, 10);
    }
  };

  const clearReconnect = (): void => {
    reconnectAttempt.value = null;
    reconnectDelayMs.value = null;
  };

  subscribe<{ attempt: number; delayMs: number }>('live.reconnecting', (data) => {
    revision += 1;
    status.value = 'retrying';
    error.value = '';
    reconnectAttempt.value = Number.isFinite(data?.attempt) ? data.attempt : null;
    reconnectDelayMs.value = Number.isFinite(data?.delayMs) ? data.delayMs : null;
  });
  subscribe<{ phase: string; message: string }>('live.error', (data) => {
    revision += 1;
    status.value = 'error';
    error.value = data.message;
    clearReconnect();
  });
  subscribe<{ uniqueId?: string | null }>('live.connected', (data) => {
    revision += 1;
    status.value = 'connected';
    error.value = '';
    clearReconnect();
    if (data.uniqueId) noteCreatorSeen(normalizeUsername(data.uniqueId), false);
  });
  subscribe('live.disconnected', () => {
    revision += 1;
    if (status.value !== 'error') status.value = 'disconnected';
    clearReconnect();
  });

  const requestConnect = async (
    target: string,
    sessionCookie: string,
    roomId?: string,
  ): Promise<void> => {
    const requestRevision = ++revision;
    try {
      const result = await control.call<LiveStatusResult>('live.connect', {
        uniqueId: target,
        sessionCookie,
        ...(roomId ? { roomId } : {}),
      });
      if (requestRevision === revision && !result.connected) status.value = 'disconnected';
    } catch (failure) {
      if (requestRevision !== revision) return;
      status.value = 'error';
      error.value = errorMessage(failure);
    }
  };

  const handleConnect = (userToConnect?: string, roomId?: string): void => {
    const target = normalizeUsername(userToConnect || uniqueId.value);
    if (!target) {
      error.value = callbacks.translate('connection.handleRequired');
      return;
    }
    error.value = '';
    if (target !== activeCreatorRef.value) callbacks.resetFeed();
    status.value = 'connecting';
    clearReconnect();
    activeCreator.value = target;
    activeCreatorRef.value = target;
    uniqueId.value = target;
    saveUsername(target);
    recents.value = addRecentUsername(target);
    callbacks.goFeed();
    void requestConnect(target, cookie.value.trim(), roomId);
  };

  const handleConnectRoom = (uniqueId: string, roomId: string): void => {
    handleConnect(uniqueId, roomId);
  };

  const handlePickLive = (): void => {
    const requestRevision = ++revision;
    error.value = '';
    callbacks.resetFeed();
    status.value = 'connecting';
    clearReconnect();
    const previousCreator = activeCreatorRef.value;
    activeCreator.value = callbacks.translate('connection.searchingRooms');
    callbacks.goFeed();
    void control
      .call<LiveStatusResult>('live.pick', { sessionCookie: cookie.value.trim() })
      .then((result) => {
        if (requestRevision !== revision) return;
        if (!result.connected) {
          status.value = 'disconnected';
          activeCreator.value = previousCreator;
        } else if (result.uniqueId) {
          uniqueId.value = normalizeUsername(result.uniqueId);
        }
      })
      .catch((failure: unknown) => {
        if (requestRevision !== revision) return;
        status.value = 'error';
        error.value = errorMessage(failure);
        activeCreator.value = previousCreator;
      });
  };

  const handleDisconnect = (): void => {
    const requestRevision = ++revision;
    void control.call('live.disconnect', {}).catch((failure: unknown) => {
      if (requestRevision !== revision) return;
      error.value = errorMessage(failure);
    });
    status.value = 'disconnected';
    error.value = '';
    activeCreator.value = activeCreatorRef.value;
    clearReconnect();
  };

  const handleReconnect = (): void => {
    if (activeCreatorRef.value) handleConnect(activeCreatorRef.value);
  };

  const handleSelectRecent = (username: string): void => {
    uniqueId.value = username;
    handleConnect(username);
  };

  const setUniqueId = (value: string): void => {
    uniqueId.value = value;
  };
  const setCookie = (value: string): void => {
    cookie.value = value;
  };

  const refreshStatus = async (): Promise<void> => {
    const requestRevision = revision;
    const refresh = ++statusRefresh;
    try {
      const result = await control.call<LiveStatusResult>('live.status', {});
      if (requestRevision !== revision || refresh !== statusRefresh) return;
      if (result.connected) {
        status.value = 'connected';
        clearReconnect();
        if (result.uniqueId) noteCreatorSeen(normalizeUsername(result.uniqueId), false);
      } else {
        status.value = 'disconnected';
        clearReconnect();
      }
    } catch (failure) {
      if (requestRevision === revision && refresh === statusRefresh) console.warn(`live status refresh failed: ${errorMessage(failure)}`);
    }
  };

  return {
    uniqueId,
    cookie,
    activeCreator,
    recents,
    status,
    error,
    reconnectAttempt,
    reconnectDelayMs,
    maxReconnectAttempts: MAX_RECONNECT_ATTEMPTS,
    noteCreatorSeen,
    mergeRecentNames,
    handleConnect,
    handleConnectRoom,
    handlePickLive,
    handleDisconnect,
    handleReconnect,
    handleSelectRecent,
    setUniqueId,
    setCookie,
    refreshStatus,
    dispose: () => { revision += 1; while (unsubscribers.length > 0) unsubscribers.pop()?.(); },
  };
}
