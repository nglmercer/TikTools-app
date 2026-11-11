import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useCloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import {
  connectWebhook,
  disconnect,
  listConnections,
  readEvents,
  type IntegrationConnection,
  type IntegrationConnectResult,
  type IntegrationEventList,
} from '../api/integrations-api.ts';

const LABEL_MAX = 120;

/** Same bounds the host and the server apply: 1 to 120 characters, no control characters. */
export function isValidLabel(value: string): boolean {
  const label = value.trim();
  if (label.length === 0 || [...label].length > LABEL_MAX) return false;
  return ![...label].some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code < 0x20 || code === 0x7f;
  });
}

/**
 * Cloud integration connections for the signed-in account. The webhook secret
 * is held in memory only: it is shown once, then cleared on dismiss or unmount.
 */
export const useIntegrationsStore = defineStore('cloud-integrations', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();

  const connections = ref<readonly IntegrationConnection[]>([]);
  const loading = ref(false);
  const working = ref(false);
  const error = ref<string | null>(null);
  const { detail: errorDetail, capture } = useCloudFailure();
  const issued = ref<IntegrationConnectResult | null>(null);
  const selectedId = ref<string | null>(null);
  const events = ref<IntegrationEventList | null>(null);
  const eventsError = ref<string | null>(null);
  let listGeneration = 0;
  let eventGeneration = 0;

  /** Undefined until the host announces the methods; never treated as absent. */
  const available = computed(() => capabilities.supports('integrations.connections'));

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    const ticket = ++listGeneration;
    loading.value = true;
    try {
      const next = await listConnections(client);
      if (ticket === listGeneration) connections.value = next;
      if (ticket === listGeneration) error.value = null;
    } catch (failure) {
      if (ticket === listGeneration) error.value = capture(failure);
    } finally {
      if (ticket === listGeneration) loading.value = false;
    }
  }

  /** Creates a webhook connection. The label must already satisfy `isValidLabel`. */
  async function connect(label: string): Promise<boolean> {
    working.value = true;
    error.value = null;
    try {
      issued.value = await connectWebhook(client, label.trim());
      await refresh();
      return true;
    } catch (failure) {
      error.value = capture(failure);
      return false;
    } finally {
      working.value = false;
    }
  }

  function dismissSecret(): void {
    issued.value = null;
  }

  async function openEvents(id: string): Promise<void> {
    const ticket = ++eventGeneration;
    selectedId.value = id;
    events.value = null;
    eventsError.value = null;
    try {
      const next = await readEvents(client, id);
      if (ticket === eventGeneration) events.value = next;
    } catch (failure) {
      if (ticket === eventGeneration) eventsError.value = errorMessage(failure);
    }
  }

  function closeEvents(): void {
    eventGeneration += 1;
    selectedId.value = null;
    events.value = null;
    eventsError.value = null;
  }

  /** Revokes a connection. Later deliveries are refused; stored events stay readable. */
  async function remove(id: string): Promise<boolean> {
    working.value = true;
    error.value = null;
    try {
      await disconnect(client, id);
      if (selectedId.value === id) closeEvents();
      await refresh();
      return true;
    } catch (failure) {
      error.value = capture(failure);
      return false;
    } finally {
      working.value = false;
    }
  }

  /** Drops the secret and any open event view, for example when the panel closes. */
  function reset(): void {
    issued.value = null;
    closeEvents();
  }

  return {
    connections,
    loading,
    working,
    error,
    errorDetail,
    issued,
    selectedId,
    events,
    eventsError,
    available,
    refresh,
    connect,
    dismissSecret,
    openEvents,
    closeEvents,
    remove,
    reset,
  };
});
