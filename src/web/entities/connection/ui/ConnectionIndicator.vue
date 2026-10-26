<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ConnectionStatus } from '@/types.ts';
import { connectionStatusLabel, connectionTone } from '../lib/connection-status.ts';

const props = defineProps<{
  status: ConnectionStatus;
  reconnectAttempt?: number | null;
  reconnectDelayMs?: number | null;
  maxReconnectAttempts?: number;
}>();

const { t } = useI18n();
const tone = computed(() => connectionTone(props.status));
const label = computed(() => {
  const { key, params } = connectionStatusLabel(props.status, props.reconnectAttempt, props.maxReconnectAttempts, props.reconnectDelayMs);
  return t(key, params ?? {});
});
</script>

<template>
  <span
    class="connection-indicator"
    :class="`connection-indicator--${tone}`"
    role="status"
  >
    <span class="connection-indicator__dot" />
    {{ label }}
  </span>
</template>

<style scoped>
.connection-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  white-space: nowrap;
}

.connection-indicator__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-subtle);
}

.connection-indicator--online .connection-indicator__dot {
  background: var(--color-success);
}

.connection-indicator--busy .connection-indicator__dot {
  background: var(--color-warning);
  animation: connection-indicator-pulse 1.2s ease-in-out infinite;
}

.connection-indicator--error .connection-indicator__dot {
  background: var(--color-danger);
}

@keyframes connection-indicator-pulse {
  50% {
    opacity: 0.35;
  }
}

@media (prefers-reduced-motion: reduce) {
  .connection-indicator--busy .connection-indicator__dot {
    animation: none;
  }
}
</style>
