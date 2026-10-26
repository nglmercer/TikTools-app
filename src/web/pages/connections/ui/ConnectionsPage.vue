<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { LiveChannel } from '@/types.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import LiveConnectPanel from '@/features/connect-live/ui/LiveConnectPanel.vue';
import LiveExplorerPanel from '@/features/explore-live-rooms/ui/LiveExplorerPanel.vue';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import PluginServersPanel from '@/widgets/plugin-servers/ui/PluginServersPanel.vue';

const connection = useConnectionStore();
const discovery = useDiscoveryStore();
const saved = loadViewState().connections;
const useCookie = ref(saved?.showCookie ?? Boolean(connection.cookie));
const expandedServers = ref<Record<string, boolean>>(saved?.openServers ?? {});

const handle = computed({
  get: () => connection.uniqueId,
  set: (value: string) => connection.setUniqueId(value),
});

const cookie = computed({
  get: () => connection.cookie,
  set: (value: string) => connection.setCookie(value),
});

function connectRoom(room: LiveChannel): void {
  if (discovery.isStale()) {
    discovery.markStale();
    discovery.refresh();
    return;
  }
  connection.handleConnectRoom(room.uniqueId, room.roomId);
}

const persist = debounce(() => {
  saveViewState('connections', { showCookie: useCookie.value, openServers: expandedServers.value });
});
watch([useCookie, expandedServers], persist, { deep: true });
</script>

<template>
  <div class="connections-page">
    <div class="connections-page__top">
      <LiveConnectPanel
        v-model:handle="handle"
        v-model:cookie="cookie"
        v-model:use-cookie="useCookie"
        class="connections-page__live"
        :status="connection.status"
        :monitor-enabled="connection.monitorStatus?.enabled"
        :monitor-phase="connection.monitorStatus?.phase"
        :monitor-busy="connection.monitorBusy"
        :monitor-error="connection.monitorError || connection.monitorStatus?.lastError || ''"
        :creator="connection.activeCreator"
        :recents="connection.recents"
        :error="connection.error"
        :reconnect-attempt="connection.reconnectAttempt"
        :reconnect-delay-ms="connection.reconnectDelayMs"
        :max-reconnect-attempts="connection.maxReconnectAttempts"
        @start-monitor="connection.startMonitor(connection.uniqueId, connection.cookie)"
        @stop-monitor="connection.stopMonitor()"
        @connect="connection.handleConnect()"
        @disconnect="connection.handleDisconnect()"
        @reconnect="connection.handleReconnect()"
        @pick-random="connection.handlePickLive()"
        @select-recent="connection.handleSelectRecent"
      />
      <LiveExplorerPanel
        class="connections-page__explorer"
        :current-creator="connection.activeCreator"
        @connect="connectRoom"
      />
    </div>
    <PluginServersPanel v-model:expanded="expandedServers" />
  </div>
</template>

<style scoped>
.connections-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
  overflow-y: auto;
}

.connections-page__top {
  display: grid;
  grid-template-columns: minmax(360px, 1fr) minmax(0, 1.5fr);
  gap: var(--space-4);
  min-height: 360px;
}

@media (width < 1100px) {
  .connections-page__top {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
