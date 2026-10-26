<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import ConnectionIndicator from '@/entities/connection/ui/ConnectionIndicator.vue';
import ConnectionControls from '@/features/connection-controls/ui/ConnectionControls.vue';
import type { ConnectionStatus } from '@/types.ts';

defineProps<{
  status: ConnectionStatus;
  creator: string;
  viewers: number;
  reconnectAttempt?: number | null;
  reconnectDelayMs?: number | null;
  maxReconnectAttempts?: number;
}>();

const emit = defineEmits<{
  connect: [];
  disconnect: [];
  reconnect: [];
}>();

const { t, n } = useI18n();
</script>

<template>
  <footer class="app-status-bar">
    <ConnectionIndicator
      :status="status"
      :reconnect-attempt="reconnectAttempt"
      :reconnect-delay-ms="reconnectDelayMs"
      :max-reconnect-attempts="maxReconnectAttempts"
    />
    <span
      v-if="creator"
      class="app-status-bar__segment"
    >@{{ creator.replace(/^@/, '') }}</span>
    <span
      v-if="status === 'connected'"
      class="app-status-bar__segment"
    >{{ t('connection.viewers', { count: n(viewers) }) }}</span>
    <span class="app-status-bar__spacer" />
    <ConnectionControls
      :status="status"
      @connect="emit('connect')"
      @disconnect="emit('disconnect')"
      @reconnect="emit('reconnect')"
    />
  </footer>
</template>

<style scoped>
.app-status-bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-5);
  height: 30px;
  padding: 0 var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
}

.app-status-bar__segment {
  padding-left: var(--space-5);
  border-left: var(--border-width) solid var(--color-border);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.app-status-bar__spacer {
  flex: 1;
}
</style>
