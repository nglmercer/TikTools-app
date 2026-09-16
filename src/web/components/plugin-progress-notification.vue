<script setup lang="ts">
import { computed } from 'vue';
import type { HostMessage } from '../../shared/messages.ts';
import { IconClose } from './icons/index.ts';

type PluginProgressMessage = Extract<HostMessage, { type: 'plugin-progress' }>;

const props = defineProps<{
  notification: PluginProgressMessage;
  onDismiss: () => void;
}>();

const percentage = computed(() => {
  if (props.notification.progress === undefined) return undefined;
  return Math.round(Math.max(0, Math.min(1, props.notification.progress)) * 100);
});
</script>

<template>
  <aside
    class="plugin-progress-notification"
    :class="`is-${notification.state}`"
    role="status"
    aria-live="polite"
  >
    <div class="plugin-progress-notification__head">
      <strong>{{ notification.pluginId }}</strong>
      <button type="button" aria-label="Dismiss" @click="onDismiss"><IconClose :size="10" /></button>
    </div>
    <div class="plugin-progress-notification__status">
      <span>{{ notification.state }}</span>
      <span v-if="percentage !== undefined">{{ percentage }}%</span>
    </div>
    <p>{{ notification.message }}</p>
    <div v-if="percentage !== undefined" class="plugin-progress-notification__track">
      <div class="plugin-progress-notification__bar" :style="{ width: `${percentage}%` }" />
    </div>
    <div v-else class="plugin-progress-notification__indeterminate" />
  </aside>
</template>

<style scoped>
.plugin-progress-notification {
  position: fixed;
  top: 62px;
  right: 18px;
  z-index: 80;
  width: min(360px, calc(100vw - 36px));
  padding: 12px 14px;
  border: 1px solid var(--line-focus);
  border-radius: 12px;
  background: var(--panel-solid);
  color: var(--text);
  box-shadow: var(--card-shadow), var(--glow-cyan);
}

.plugin-progress-notification.is-failed {
  border-color: color-mix(in srgb, var(--tt-danger) 65%, var(--line));
  box-shadow: var(--card-shadow), 0 0 20px rgba(239, 71, 111, 0.2);
}

.plugin-progress-notification.is-ready {
  border-color: color-mix(in srgb, var(--tt-green) 65%, var(--line));
}

.plugin-progress-notification__head,
.plugin-progress-notification__status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.plugin-progress-notification__head {
  font-size: 12px;
}

.plugin-progress-notification__head button {
  border: 0;
  padding: 0 2px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
}

.plugin-progress-notification__head button:hover {
  color: var(--text);
}

.plugin-progress-notification__status {
  margin-top: 8px;
  color: var(--tt-cyan);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.is-failed .plugin-progress-notification__status {
  color: var(--tt-danger);
}

.is-ready .plugin-progress-notification__status {
  color: var(--tt-green);
}

.plugin-progress-notification p {
  margin: 7px 0 10px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.4;
}

.plugin-progress-notification__track,
.plugin-progress-notification__indeterminate {
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--bg-subtle);
}

.plugin-progress-notification__bar {
  height: 100%;
  border-radius: inherit;
  background: var(--brand-gradient);
  transition: width 180ms ease;
}

.plugin-progress-notification__indeterminate::after {
  display: block;
  width: 38%;
  height: 100%;
  border-radius: inherit;
  background: var(--brand-gradient);
  content: '';
  animation: plugin-progress-slide 1.2s ease-in-out infinite;
}

@keyframes plugin-progress-slide {
  from { transform: translateX(-100%); }
  to { transform: translateX(270%); }
}
</style>
