<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionStatusNode } from '../../../../plugin-ui/index.ts';
import type { PluginUiContext } from '@/entities/plugin/model/plugin-ui-context.ts';
import type { PluginActionOutcome } from '@/entities/plugin/model/plugins-state.ts';
import { toSettingValues } from '../../../../shared/settings-values.ts';

const props = defineProps<{
  node: ActionStatusNode;
  context: PluginUiContext;
}>();

const { t } = useI18n();
const outcome = ref<PluginActionOutcome>();
const refreshing = ref(false);
let timer: number | undefined;
let disposed = false;
let inflight = false;

type StatusResult = {
  phase?: string;
  workerRunning?: boolean;
  channel?: string;
  message?: string;
  error?: { code?: string; message?: string };
  reconnectAttempt?: number;
  retryAfterMs?: number;
  updatedAt?: number;
};

const result = computed<StatusResult | undefined>(() => {
  const value = outcome.value?.result;
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as StatusResult) : undefined;
});
const phase = computed(() => result.value?.phase ?? (outcome.value?.ok ? 'unknown' : 'error'));
const tone = computed(() => {
  const value = phase.value.toLowerCase();
  if (value === 'connected' || value === 'ready' || value === 'ok') return 'ok';
  if (value === 'failed' || value === 'error') return 'error';
  return 'info';
});
const headline = computed(() => {
  if (outcome.value?.error) return outcome.value.error;
  return result.value?.message ?? outcome.value?.summary ?? t('plugins.page.listEmpty');
});
const details = computed(() => {
  const status = result.value;
  if (!status) return [];
  const rows: Array<{ key: string; value: string }> = [];
  if (status.channel) rows.push({ key: 'channel', value: status.channel });
  rows.push({ key: 'state', value: status.phase ?? 'unknown' });
  if (typeof status.workerRunning === 'boolean') {
    rows.push({ key: 'worker', value: t(status.workerRunning ? 'plugins.page.status.workerRunning' : 'plugins.page.status.workerStopped') });
  }
  if (status.error?.message) rows.push({ key: 'error', value: status.error.message });
  if (status.reconnectAttempt) rows.push({ key: 'reconnectAttempt', value: String(status.reconnectAttempt) });
  if (status.retryAfterMs) rows.push({ key: 'retryAfter', value: `${Math.ceil(status.retryAfterMs / 1000)}s` });
  if (status.updatedAt) rows.push({ key: 'updated', value: new Date(status.updatedAt).toLocaleTimeString() });
  return rows;
});

async function refresh(): Promise<void> {
  if (inflight || disposed) return;
  inflight = true;
  refreshing.value = true;
  try {
    const next = await props.context.actions.execute(props.node.action.actionType, toSettingValues(props.node.action.config ?? {}));
    if (!disposed) {
      outcome.value = next;
      if (props.node.stateKey) props.context.local.set(props.node.stateKey, phase.value);
    }
  } finally {
    inflight = false;
    refreshing.value = false;
  }
}

onMounted(() => {
  void refresh();
  timer = window.setInterval(() => void refresh(), props.node.pollMs ?? 2_500);
});

onBeforeUnmount(() => {
  disposed = true;
  if (timer !== undefined) window.clearInterval(timer);
});
</script>

<template>
  <section
    class="plugin-action-status"
    :class="`plugin-action-status--${tone}`"
    aria-live="polite"
  >
    <header class="plugin-action-status__header">
      <span
        class="plugin-action-status__dot"
        aria-hidden="true"
      />
      <strong>{{ headline }}</strong>
      <span
        v-if="refreshing && outcome"
        class="plugin-action-status__muted"
      >…</span>
    </header>
    <dl
      v-if="details.length"
      class="plugin-action-status__details"
    >
      <template
        v-for="row in details"
        :key="row.key"
      >
        <dt>{{ t(`plugins.page.status.${row.key}`) }}</dt>
        <dd>{{ row.value }}</dd>
      </template>
    </dl>
  </section>
</template>

<style scoped>
.plugin-action-status {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, currentColor 4%, transparent);
}

.plugin-action-status--ok {
  color: var(--color-success, #3fb950);
}

.plugin-action-status--error {
  color: var(--color-danger, #f85149);
}

.plugin-action-status--info {
  color: var(--color-accent, #58a6ff);
}

.plugin-action-status__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.plugin-action-status__header strong {
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.plugin-action-status__dot {
  width: 0.625rem;
  height: 0.625rem;
  flex: 0 0 auto;
  border-radius: 999px;
  background: currentColor;
  box-shadow: 0 0 0 0.25rem color-mix(in srgb, currentColor 14%, transparent);
}

.plugin-action-status__details {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-1) var(--space-3);
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.plugin-action-status__details dt {
  font-weight: var(--font-weight-medium);
}

.plugin-action-status__details dd {
  margin: 0;
  color: var(--color-text);
  overflow-wrap: anywhere;
}

.plugin-action-status__muted {
  color: var(--color-text-muted);
}
</style>
