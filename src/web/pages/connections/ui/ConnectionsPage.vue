<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { LiveChannel } from '@/types.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import LiveConnectPanel from '@/features/connect-live/ui/LiveConnectPanel.vue';
import PlatformConnectionsPanel from '@/features/connect-live/ui/PlatformConnectionsPanel.vue';
import LiveExplorerPanel from '@/features/explore-live-rooms/ui/LiveExplorerPanel.vue';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import PluginServersPanel from '@/widgets/plugin-servers/ui/PluginServersPanel.vue';

const connection = useConnectionStore();
const discovery = useDiscoveryStore();
const saved = loadViewState().connections;
const useCookie = ref(saved?.showCookie ?? Boolean(connection.cookie));
// Only one server is open at a time; the view state keeps the existing `openServers` shape.
const selectedServer = ref<string | null>(Object.entries(saved?.openServers ?? {}).find(([, open]) => open)?.[0] ?? null);
const autoConnectOption = ref(false);
const autoConnect = computed({
  get: () => Boolean(connection.monitorStatus?.enabled) || autoConnectOption.value,
  set: (value: boolean) => {
    autoConnectOption.value = value;
    if (!value && connection.monitorStatus?.enabled) void connection.stopMonitor();
  },
});

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

function connect(): void {
  if (autoConnect.value) void connection.startMonitor(connection.uniqueId, useCookie.value ? connection.cookie : '');
  else connection.handleConnect(undefined, undefined, useCookie.value ? connection.cookie : '');
}

function selectRecent(username: string): void {
  connection.setUniqueId(username);
  connect();
}

const persist = debounce(() => {
  saveViewState('connections', {
    showCookie: useCookie.value,
    openServers: selectedServer.value ? { [selectedServer.value]: true } : {},
  });
});
watch([useCookie, selectedServer], persist);
</script>

<template>
  <div
    class="connections-page"
    data-guide-scope="connections"
  >
    <div class="connections-page__top">
      <LiveConnectPanel
        v-model:handle="handle"
        v-model:cookie="cookie"
        v-model:use-cookie="useCookie"
        v-model:auto-connect="autoConnect"
        data-guide="connection-form"
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
        @stop-monitor="autoConnect = false"
        @connect="connect"
        @disconnect="connection.handleDisconnect()"
        @reconnect="connection.handleReconnect()"
        @pick-random="connection.handlePickLive()"
        @select-recent="selectRecent"
      />
      <LiveExplorerPanel
        data-guide="connection-explorer"
        class="connections-page__explorer"
        :current-creator="connection.activeCreator"
        @connect="connectRoom"
      />
    </div>
    <PlatformConnectionsPanel />
    <PluginServersPanel
      v-model:selected="selectedServer"
      data-guide="connection-plugins"
      class="connections-page__servers"
    />
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
  flex: none;
  grid-template-columns: minmax(360px, 1fr) minmax(0, 1.5fr);
  gap: var(--space-4);
  min-height: 360px;
}

/* The lower panel is where navigation happens (list -> server settings), so it takes the remaining height. */
.connections-page__servers {
  flex: 1 0 auto;
  min-height: 260px;
}

.connections-page :deep(.connections-page__live > .base-panel__body),
.connections-page :deep(.connections-page__explorer > .base-panel__body) {
  overflow: visible;
}

@media (width < 1100px) {
  .connections-page__top {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
