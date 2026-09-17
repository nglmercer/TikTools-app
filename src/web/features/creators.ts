import { ref } from 'vue';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import type { CreatorRecord } from '../types.ts';

export interface CreatorsCallbacks {
  noteCreatorSeen: (clean: string, persist: boolean) => void;
  mergeRecentNames: (names: string[]) => void;
}

export interface CreatorGetResult {
  creator: CreatorRecord | null;
}

export interface CreatorsRecentResult {
  creators: CreatorRecord[];
}

function normalizeUsername(value: string): string {
  return value.trim().replace(/^@/, '');
}

/** Creator records and history. */
export function useCreators(control: ControlClient, callbacks: CreatorsCallbacks) {
  const activeCreatorRecord = ref<CreatorRecord | null>(null);
  const recentCreators = ref<CreatorRecord[]>([]);

  control.onTopic('creator.changed', () => {
    void refresh().catch((failure: unknown) => {
      console.warn(`creators refresh failed: ${errorMessage(failure)}`);
    });
  });

  const refresh = async (): Promise<void> => {
    const [current, recent] = await Promise.all([
      control.call<CreatorGetResult>('creators.get', {}),
      control.call<CreatorsRecentResult>('creators.recent', { limit: 10 }),
    ]);
    activeCreatorRecord.value = current.creator;
    recentCreators.value = recent.creators;
    if (current.creator?.uniqueId) {
      callbacks.noteCreatorSeen(normalizeUsername(current.creator.uniqueId), true);
    }
    callbacks.mergeRecentNames(recent.creators.map((creator) => creator.uniqueId));
  };

  const refreshQuiet = async (): Promise<void> => {
    try {
      await refresh();
    } catch (failure) {
      console.warn(`creators refresh failed: ${errorMessage(failure)}`);
    }
  };

  return {
    activeCreatorRecord,
    recentCreators,
    refresh: refreshQuiet,
  };
}
