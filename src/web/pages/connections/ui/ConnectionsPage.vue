<script setup lang="ts">
import { useI18n } from "vue-i18n";
import BaseDisclosure from "@/shared/ui/disclosure/BaseDisclosure.vue";
import { computed, ref, watch } from 'vue';
import type { LiveChannel } from '@/types.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import LiveConnectPanel from '@/features/connect-live/ui/LiveConnectPanel.vue';
import LiveExplorerPanel from '@/features/explore-live-rooms/ui/LiveExplorerPanel.vue';
import { debounce, loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import PluginServersPanel from '@/widgets/plugin-servers/ui/PluginServersPanel.vue';

const { t } = useI18n();
const explorerExpanded = ref(false);
const connection = useConnectionStore();
const discovery = useDiscoveryStore();
const saved = loadViewState().connections;
const useCookie = ref(saved?.showCookie ?? Boolean(connection.cookie));
const expandedServers = ref<Record<string, boolean>>(saved?.openServers ?? {});
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
  saveViewState('connections', { showCookie: useCookie.value, openServers: expandedServers.value });
});
watch([useCookie, expandedServers], persist, { deep: true });
</script>

<template>
  <div class="connections-page">
    <header class="connections-page__heading">
      <h1>{{ t('settingsLayout.connections') }}</h1><p>{{ t('settingsLayout.connectionsHint') }}</p>
    </header>
    <div class="connections-page__top">
      <LiveConnectPanel
        v-model:handle="handle"
        v-model:cookie="cookie"
        v-model:use-cookie="useCookie"
        v-model:auto-connect="autoConnect"
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
    </div>
    <PluginServersPanel
      v-model:expanded="expandedServers"
      class="connections-page__servers"
    />
    <BaseDisclosure
      v-model:open="explorerExpanded"
      :title="t('settingsLayout.explorer')"
    >
      <LiveExplorerPanel
        class="connections-page__explorer"
        :current-creator="connection.activeCreator"
        @connect="connectRoom"
      />
    </BaseDisclosure>
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
  padding: var(--space-6);
  max-width: 1000px;
  overflow-y: auto;
}

.connections-page__top {
  display: grid;
  flex: none;
  grid-template-columns: minmax(0, 1fr);
  gap: var(--space-4);
  min-height: 0;
}

.connections-page__servers {
  flex: none;
}

.connections-page :deep(.connections-page__live > .base-panel__body),
.connections-page :deep(.connections-page__explorer > .base-panel__body),
.connections-page :deep(.connections-page__servers > .base-panel__body) {
  overflow: visible;
}

@media (width < 1100px) {
  .connections-page__top {
    grid-template-columns: minmax(0, 1fr);
  }
}

.connections-page__heading h1 { margin:0; font-size:20px; }
.connections-page__heading p { color:var(--color-text-muted); line-height:var(--line-height-normal); }
.connections-page :deep(.base-panel) { background:transparent; border:0; border-top:var(--border-width) solid var(--color-border); }
.connections-page :deep(.base-panel__header) { background:transparent; padding:var(--space-5) 0; }
.connections-page :deep(.base-panel__body) { padding:var(--space-5) 0; }
</style>
