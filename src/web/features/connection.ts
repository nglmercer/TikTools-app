import { ref } from 'vue';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import {
  addRecentUsername,
  getRecentUsernames,
  getSavedUsername,
  saveUsername,
} from '../preferences.ts';
import type { ConnectionStatus } from '../types.ts';

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

/** Live connection state: connect form, status, and active creator. */
export function useConnection(control: ControlClient, callbacks: ConnectionCallbacks) {
  const uniqueId = ref(getSavedUsername());
  const cookie = ref('');
  const activeCreator = ref(getSavedUsername());
  const activeCreatorRef = ref(getSavedUsername());
  const recents = ref<string[]>(getRecentUsernames());
  const status = ref<ConnectionStatus>('idle');
  const error = ref('');

  const noteCreatorSeen = (clean: string, persist: boolean): void => {
    activeCreator.value = clean;
    activeCreatorRef.value = clean;
    recents.value = addRecentUsername(clean);
    if (persist) saveUsername(clean);
  };

  const mergeRecentNames = (names: string[]): void => {
    if (names.length > 0) {
      recents.value = [...new Set([...names, ...recents.value])].slice(0, 10);
    }
  };

  control.onPush('reconnecting', () => {
    status.value = 'retrying';
  });
  control.onPush('error', (message) => {
    if (message.type !== 'error') return;
    status.value = 'error';
    error.value = message.message;
  });
  control.onTopic<{ uniqueId?: string | null }>('live.connected', (data) => {
    status.value = 'connected';
    if (data.uniqueId) noteCreatorSeen(normalizeUsername(data.uniqueId), false);
  });
  control.onTopic('live.disconnected', () => {
    status.value = 'disconnected';
  });

  const requestConnect = async (target: string, sessionCookie: string): Promise<void> => {
    try {
      const result = await control.call<LiveStatusResult>('live.connect', {
        uniqueId: target,
        sessionCookie,
      });
      if (!result.connected) status.value = 'disconnected';
    } catch (failure) {
      status.value = 'error';
      error.value = errorMessage(failure);
    }
  };

  const handleConnect = (userToConnect?: string): void => {
    const target = normalizeUsername(userToConnect || uniqueId.value);
    if (!target) {
      error.value = callbacks.translate('handleRequired');
      return;
    }
    error.value = '';
    callbacks.resetFeed();
    status.value = 'connecting';
    activeCreator.value = target;
    activeCreatorRef.value = target;
    saveUsername(target);
    recents.value = addRecentUsername(target);
    callbacks.goFeed();
    void requestConnect(target, cookie.value.trim());
  };

  const handlePickLive = (): void => {
    error.value = '';
    callbacks.resetFeed();
    status.value = 'connecting';
    const previousCreator = activeCreatorRef.value;
    activeCreator.value = callbacks.translate('searchingRooms');
    callbacks.goFeed();
    void control
      .call<LiveStatusResult>('live.pick', { sessionCookie: cookie.value.trim() })
      .then((result) => {
        if (!result.connected) {
          status.value = 'disconnected';
          activeCreator.value = previousCreator;
        }
      })
      .catch((failure: unknown) => {
        status.value = 'error';
        error.value = errorMessage(failure);
        activeCreator.value = previousCreator;
      });
  };

  const handleDisconnect = (): void => {
    void control.call('live.disconnect', {}).catch((failure: unknown) => {
      error.value = errorMessage(failure);
    });
    status.value = 'disconnected';
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

  return {
    uniqueId,
    cookie,
    activeCreator,
    recents,
    status,
    error,
    noteCreatorSeen,
    mergeRecentNames,
    handleConnect,
    handlePickLive,
    handleDisconnect,
    handleReconnect,
    handleSelectRecent,
    setUniqueId,
    setCookie,
  };
}
