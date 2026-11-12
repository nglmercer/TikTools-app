<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
const props = defineProps<{ connection?: PluginConnectionState; guided: boolean; dirty: boolean }>();
const { t } = useI18n();
const serverInfo = computed(() => props.connection?.serverInfo);
</script>

<template>
  <p
    v-if="guided && serverInfo && !dirty"
    class="connection-editor__note"
    role="status"
  >
    {{ t(`plugins.connection.model.${serverInfo.modelStatus}`) }}
    <span v-if="!serverInfo.authRequired"> · {{ t('plugins.connection.noTokenNeeded') }}</span>
  </p>
  <p
    v-if="guided && serverInfo && !serverInfo.playback && !dirty"
    class="connection-editor__note"
  >
    {{ t('plugins.connection.noPlayback') }}
  </p>
  <p
    v-if="connection?.gameplayReady !== undefined && !dirty"
    class="connection-editor__note"
  >
    {{ connection.gameplayReady ? t('plugins.connection.gameplayReady') : t('connections.servers.apiNotReady') }}
  </p>
</template>
