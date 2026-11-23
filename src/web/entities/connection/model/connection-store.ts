import { defineStore } from 'pinia';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import { createLiveMonitorState } from './live-monitor-state.ts';
import { useConnection } from './connection-state.ts';
import { useCreators } from './creators-state.ts';

export const useConnectionStore = defineStore('connection', () => {
  const client = useControlClient();
  const navigation = useNavigationStore();
  const live = useLiveFeedStore();
  const connection = useConnection(client, {
    goFeed: () => navigation.open('feed'),
    resetFeed: () => live.resetEvents(),
    translate: (key) => i18n.global.t(key),
  });
  const monitor = createLiveMonitorState(client);
  const creators = useCreators(client, {
    noteCreatorSeen: (clean, persist) => connection.noteCreatorSeen(clean, persist),
    mergeRecentNames: (names) => connection.mergeRecentNames(names),
  });

  return {
    ...connection,
    ...monitor,
    dispose: () => { connection.dispose(); monitor.disposeMonitor(); },
    activeCreatorRecord: creators.activeCreatorRecord,
    recentCreators: creators.recentCreators,
    refreshCreators: creators.refresh,
  };
});
