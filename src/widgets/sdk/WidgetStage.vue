<script setup lang="ts">
import type { GatewayStatus } from '../shared/gateway-client.ts';
withDefaults(defineProps<{ align?: 'center' | 'bottom'; debug: boolean; status: GatewayStatus }>(), { align: 'center' });
</script>

<template>
  <div class="widget-stage" :class="{ 'is-bottom': align === 'bottom' }">
    <slot />
    <div v-if="debug" class="widget-debug-status">gateway: {{ status }}</div>
  </div>
</template>

<style scoped>
.widget-stage {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  user-select: none;
  font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
}
.widget-stage.is-bottom { align-items: flex-end; }
.widget-debug-status {
  position: absolute;
  bottom: 8px;
  left: 8px;
  padding: 4px 8px;
  font: 11px monospace;
  color: #8b8b96;
  background: rgba(0, 0, 0, 0.6);
  border-radius: 6px;
}
</style>
