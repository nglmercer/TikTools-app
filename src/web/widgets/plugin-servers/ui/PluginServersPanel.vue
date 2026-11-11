<script setup lang="ts">
import { computed, type ComputedRef, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import { connectionStatusText } from '@/entities/plugin/lib/connection-status.ts';
import { usePluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconServer from '@/shared/ui/icon/icons/IconServer.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import DiagnosticStatusAction from '@/shared/diagnostics/DiagnosticStatusAction.vue';
import { serverEntries, type ServerEntry } from '../model/server-entries.ts';

/** Id of the server whose settings are open, or null for the list. Navigation happens inside this panel. */
const selected = defineModel<string | null>('selected', { default: null });

const { t } = useI18n();
const router = useRouter();
const plugins = usePluginControls();

const servers: ComputedRef<ServerEntry[]> = computed(() =>
  serverEntries(plugins.behavior.value.plugins, plugins.connections.value, (plugin) => localizedText(plugin.descriptor.name)),
);
// A server that was disabled or removed while open falls back to the list.
const current = computed(() => servers.value.find((server) => server.id === selected.value));
const editor = useTemplateRef<{ flush?: () => void }>('editor');
const testing = ref(false);
watch([() => current.value?.connection?.at, selected], () => {
  testing.value = false;
});

function testCurrent(): void {
  const server = current.value;
  if (!server) return;
  testing.value = true;
  // Save pending edits first: the probe reads the stored connection settings.
  editor.value?.flush?.();
  plugins.testConnection(server.id);
}

function statusText(connection: PluginConnectionState | undefined): string {
  return connectionStatusText(connection, t);
}
</script>

<template>
  <BasePanel
    :title="current ? current.name : t('connections.servers.title')"
    :subtitle="current ? undefined : t('connections.servers.lead')"
    flush
  >
    <template
      v-if="current"
      #leading
    >
      <BaseIconButton
        :label="t('connections.servers.back')"
        size="sm"
        @click="selected = null"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>

    <template
      v-if="current"
      #status
    >
      <DiagnosticStatusAction
        v-if="current.connection && !current.connection.ok"
        :label="statusText(current.connection)"
        source="plugins"
        :entity-id="current.id"
      />
      <BaseStatus
        v-else
        :label="statusText(current.connection)"
        :tone="current.connection?.ok ? 'success' : 'neutral'"
      />
    </template>

    <template
      v-if="current && !current.plugin.descriptor.guidedConnection"
      #actions
    >
      <BaseButton
        variant="primary"
        :disabled="testing"
        @click="testCurrent"
      >
        <template #icon>
          <IconZap />
        </template>
        {{ testing ? t('plugins.connection.testing') : t('connections.servers.test') }}
      </BaseButton>
    </template>

    <div
      v-if="current"
      :id="`server-${current.id}`"
      class="plugin-servers__detail"
    >
      <PluginConnectionEditor
        ref="editor"
        :key="current.id"
        layout="sections"
        :plugin-id="current.id"
        :plugin-name="current.name"
        :settings="plugins.settings.value[current.id]"
        :connection="current.connection"
        :action-options="plugins.actionOptions.value"
        :open-media-picker="plugins.openMediaPicker"
        :supports-token-import="current.plugin.descriptor.supportsTokenImport"
        :supports-token-discovery="current.plugin.descriptor.supportsTokenDiscovery"
        :supports-provisioning="current.plugin.descriptor.supportsTokenProvisioning"
        :guided="current.plugin.descriptor.guidedConnection"
        :provision-state="plugins.provision.value[current.id]"
        @load-settings="plugins.loadSettings"
        @load-options="plugins.loadOptions"
        @save="plugins.saveSettings"
        @test="plugins.testConnection"
        @import-token="plugins.importToken"
        @provision="plugins.provisionToken"
      />
    </div>
    <BaseEmptyState
      v-else-if="!servers.length"
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
          class="plugin-servers__open"
          type="button"
          @click="selected = server.id"
        >
          <span class="plugin-servers__name">{{ server.name }}</span>
        </button>
        <span class="plugin-servers__status">
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
        </span>
        <IconChevronRight
          class="plugin-servers__chevron"
          aria-hidden="true"
        />
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

.plugin-servers__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: var(--control-height-lg);
  padding: 0 var(--space-4);
}

.plugin-servers__item + .plugin-servers__item {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-servers__item:nth-child(even) {
  background: var(--color-bg-panel-header);
}

.plugin-servers__item:hover {
  background: var(--color-bg-hover);
}

/* The open button stretches over the whole row; the status stays clickable above it. */
.plugin-servers__open {
  flex: 1;
  min-width: 0;
  min-height: var(--control-height-lg);
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.plugin-servers__open::after {
  position: absolute;
  inset: 0;
  content: '';
}

.plugin-servers__open:focus-visible {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: calc(var(--border-width) * -1);
}

.plugin-servers__name {
  overflow-wrap: anywhere;
}

.plugin-servers__status {
  position: relative;
  z-index: 1;
  flex: none;
}

.plugin-servers__chevron {
  flex: none;
  color: var(--color-text-muted);
}

.plugin-servers__detail {
  padding: var(--space-4);
}
</style>
