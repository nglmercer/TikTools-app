<script setup lang="ts">
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
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
import { serverEntries, type ServerEntry } from '../model/server-entries.ts';

const expanded = defineModel<Record<string, boolean>>('expanded', { default: () => ({}) });

const { t } = useI18n();
const router = useRouter();
const plugins = usePluginControls();

const servers: ComputedRef<ServerEntry[]> = computed(() =>
  serverEntries(plugins.behavior.value.plugins, plugins.connections.value, (plugin) => localizedText(plugin.descriptor.name)),
);


function statusText(connection: PluginConnectionState | undefined): string {
  if (!connection) return t('connections.servers.notConfigured');
  if (connection.ok) return t('connections.servers.connectedIn', { ms: connection.latencyMs });
  if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
  return t('connections.servers.failed');
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
        <div class="plugin-servers__head">
          <button
            class="plugin-servers__toggle"
            type="button"
            :aria-expanded="expanded[server.id] ?? false"
            :aria-controls="`server-${server.id}`"
            @click="toggle(server.id)"
          >
            <IconChevronDown v-if="expanded[server.id]" />
            <IconChevronRight v-else />
            <IconServer />
            <span class="plugin-servers__name">{{ server.name }}</span>
          </button>
          <DiagnosticStatusAction
            v-if="server.connection && !server.connection.ok"
            :label="statusText(server.connection)"
            source="plugins"
            :entity-id="server.id"
          />
          <BaseStatus
            v-else
            :label="statusText(server.connection)"
            :tone="server.connection?.ok ? 'success' : 'neutral'"
          />
        </div>
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
.plugin-servers__toggle { display: flex; align-items: center; gap: var(--space-4); flex: 1; min-width: 0; border: 0; background: transparent; color: inherit; font: inherit; text-align: left; min-height: var(--control-height-lg); cursor: pointer; }
.plugin-servers__toggle:focus-visible { outline: var(--border-width) solid var(--color-border-focus); }
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
  min-height: var(--control-height-lg);
  flex-wrap: wrap;
  box-sizing: border-box;
  padding: 0 var(--space-4);
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-regular);
  text-align: left;
  outline: none;
  transform: none;
}

.plugin-servers__item:nth-child(even) { background: var(--color-bg-panel-header); }

.plugin-servers__head:hover {
  background: var(--color-bg-hover);
}

.plugin-servers__name { flex: 1; overflow-wrap: anywhere; }

.plugin-servers__body {
  padding: var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-app);
}
@media (width < 700px) { .plugin-servers__head { align-items: flex-start; flex-direction: column; padding: var(--space-3) var(--space-4); gap: var(--space-1); } .plugin-servers__toggle { width: 100%; } }
</style>
