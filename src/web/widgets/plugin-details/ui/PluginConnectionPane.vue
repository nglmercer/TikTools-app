<script setup lang="ts">
import { computed, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { PluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { connectionStatusText } from '@/entities/plugin/lib/connection-status.ts';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import DiagnosticStatusAction from '@/shared/diagnostics/DiagnosticStatusAction.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
const props = defineProps<{
  descriptor: PluginStatus['descriptor'];
  name: string;
  connection?: PluginConnectionState;
  connecting: boolean;
  plugins: Pick<PluginControls, 'settings' | 'actionOptions' | 'openMediaPicker' | 'provision' |
    'loadSettings' | 'loadOptions' | 'saveSettings' | 'testConnection' | 'importToken' | 'provisionToken'>;
}>();
const emit = defineEmits<{ test: [] }>();
const { t } = useI18n();
const id = computed(() => props.descriptor.id);
const editor = useTemplateRef<{ flush?: () => void }>('editor');
function testConnection(): void {
  editor.value?.flush?.();
  emit('test');
}
</script>

<template>
  <div class="plugin-details__strip">
    <DiagnosticStatusAction
      v-if="connection && !connection.ok"
      :label="connectionStatusText(connection, t)"
      source="plugins"
      :entity-id="id"
    />
    <BaseStatus
      v-else
      :label="connectionStatusText(connection, t)"
      :tone="connection?.ok ? 'success' : 'neutral'"
    />
    <BaseButton
      v-if="!descriptor.guidedConnection"
      class="plugin-details__test"
      variant="primary"
      :disabled="connecting"
      @click="testConnection"
    >
      <template #icon>
        <IconZap />
      </template>
      {{ connecting ? t('plugins.connection.testing') : t('connections.servers.test') }}
    </BaseButton>
  </div>
  <div class="plugin-details__pane">
    <PluginConnectionEditor
      ref="editor"
      :key="id"
      layout="sections"
      :plugin-id="id"
      :plugin-name="name"
      :settings="plugins.settings.value[id]"
      :connection="connection"
      :action-options="plugins.actionOptions.value"
      :open-media-picker="plugins.openMediaPicker"
      :supports-token-import="descriptor.supportsTokenImport"
      :supports-token-discovery="descriptor.supportsTokenDiscovery"
      :supports-provisioning="descriptor.supportsTokenProvisioning"
      :guided="descriptor.guidedConnection"
      :provision-state="plugins.provision.value[id]"
      @load-settings="plugins.loadSettings"
      @load-options="plugins.loadOptions"
      @save="plugins.saveSettings"
      @test="plugins.testConnection"
      @import-token="plugins.importToken"
      @provision="plugins.provisionToken"
    />
  </div>
</template>
