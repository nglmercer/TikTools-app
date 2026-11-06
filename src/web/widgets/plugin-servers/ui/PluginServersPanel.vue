<script setup lang="ts">
import { computed, type ComputedRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconServer from '@/shared/ui/icon/icons/IconServer.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import { serverEntries, type ServerEntry } from '../model/server-entries.ts';

const expanded = defineModel<Record<string, boolean>>('expanded', { default: () => ({}) });

const { t } = useI18n();
const router = useRouter();
const plugins = usePluginControls();

const servers: ComputedRef<ServerEntry[]> = computed(() =>
  serverEntries(plugins.behavior.value.plugins, plugins.connections.value, (plugin) => localizedText(plugin.descriptor.name)),
);

function tone(connection: PluginConnectionState | undefined): 'ok' | 'error' | 'unknown' {
  if (!connection) return 'unknown';
  return connection.ok ? 'ok' : 'error';
}

function statusText(connection: PluginConnectionState | undefined): string {
  if (!connection) return t('connections.servers.notConfigured');
  if (connection.ok) return t('connections.servers.connectedIn', { ms: connection.latencyMs });
  if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
  return connection.error || t('connections.servers.failed');
}

function toggle(id: string): void {
  expanded.value = { ...expanded.value, [id]: !expanded.value[id] };
}
</script>

<template>
  <BasePanel
    :title="t('connections.servers.title')"
    :subtitle="t('connections.servers.lead')"
    flush
  >
    <BaseEmptyState
      v-if="!servers.length"
      :title="t('connections.servers.emptyTitle')"
      :description="t('connections.servers.emptyHint')"
    >
      <template #icon>
        <IconServer :size="28" />
      </template>
      <BaseButton
        size="sm"
        @click="router.push('/plugins')"
      >
        {{ t('connections.servers.openPlugins') }}
      </BaseButton>
    </BaseEmptyState>
    <ul
      v-else
      class="plugin-servers"
    >
      <li
        v-for="server in servers"
        :key="server.id"
        class="plugin-servers__item"
      >
        <button
          v-tooltip="statusText(server.connection)"
          class="plugin-servers__head"
          type="button"
          :aria-expanded="expanded[server.id] ?? false"
          :aria-controls="`server-${server.id}`"
          @click="toggle(server.id)"
        >
          <IconChevronDown v-if="expanded[server.id]" />
          <IconChevronRight v-else />
          <IconServer />
          <span class="plugin-servers__name">{{ server.name }}</span>
          <span
            class="plugin-servers__status"
            :class="`plugin-servers__status--${tone(server.connection)}`"
          >{{ statusText(server.connection) }}</span>
        </button>
        <div
          v-show="expanded[server.id]"
          :id="`server-${server.id}`"
          class="plugin-servers__body"
        >
          <PluginConnectionEditor
            :plugin-id="server.id"
            :plugin-name="server.name"
            :settings="plugins.settings.value[server.id]"
            :connection="server.connection"
            :action-options="plugins.actionOptions.value"
            :open-media-picker="plugins.openMediaPicker"
            :supports-token-import="server.plugin.descriptor.supportsTokenImport"
            :supports-token-discovery="server.plugin.descriptor.supportsTokenDiscovery"
            :supports-provisioning="server.plugin.descriptor.supportsTokenProvisioning"
            :guided="server.plugin.descriptor.guidedConnection"
            :provision-state="plugins.provision.value[server.id]"
            @load-settings="plugins.loadSettings"
            @load-options="plugins.loadOptions"
            @save="plugins.saveSettings"
            @test="plugins.testConnection"
            @import-token="plugins.importToken"
            @provision="plugins.provisionToken"
          />
        </div>
      </li>
    </ul>
  </BasePanel>
</template>

<style scoped>
.plugin-servers {
  margin: 0;
  padding: 0;
  list-style: none;
}

.plugin-servers__item + .plugin-servers__item {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-servers__head {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-4);
  width: 100%;
  height: var(--control-height-lg);
  padding: 0 var(--space-4);
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-regular);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.plugin-servers__head:hover {
  background: var(--color-bg-hover);
}

.plugin-servers__head:focus-visible {
  box-shadow: inset var(--focus-ring);
}

.plugin-servers__name {
  flex: 1;
}

.plugin-servers__status {
  max-width: 50%;
  overflow: hidden;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-servers__status--ok {
  color: var(--color-success);
}

.plugin-servers__status--error {
  color: var(--color-danger-hover);
}

.plugin-servers__body {
  padding: var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-app);
}
</style>
