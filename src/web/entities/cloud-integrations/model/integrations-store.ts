import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useCloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createActivityCounter, createRequestScope } from '@/shared/lib/request-scope.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
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
 * is held in memory only: it is shown once, then cleared on dismiss or unmount, and a
 * reply that arrives after the panel closed or the account changed never publishes it
 * (the connection itself was still created and can be revoked). Lists, one event view
 * and the operations each answer only for the generation that started them.
 */
export const useIntegrationsStore = defineStore('cloud-integrations', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();

  const connections = ref<readonly IntegrationConnection[]>([]);
  const loading = ref(false);
  const activity = createActivityCounter();
  const working = activity.active;
  const error = ref<string | null>(null);
  const { detail: errorDetail, capture, clear: clearDetail } = useCloudFailure();
  const issued = ref<IntegrationConnectResult | null>(null);
  const selectedId = ref<string | null>(null);
  const events = ref<IntegrationEventList | null>(null);
  const eventsError = ref<string | null>(null);
  const listScope = createRequestScope();
  const eventScope = createRequestScope();
  const secretScope = createRequestScope();
  const operationScope = createRequestScope();
  // Set while a panel is showing this account's data, so a new context reloads it.
  let shown = false;

  function clearError(): void {
    error.value = null;
    clearDetail();
  }

  /** A new account, session or host starts empty: no earlier list, secret, error or late reply survives. */
  useCloudContext(() => {
    const reload = shown;
    reset();
    connections.value = [];
    clearError();
    if (reload) void refresh();
  });

  /** Undefined until the host announces the methods; never treated as absent. */
  const available = computed(() => capabilities.supports('integrations.connections'));

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    shown = true;
    const ticket = listScope.begin();
    loading.value = true;
    try {
      const next = await listConnections(client);
      if (!ticket.isCurrent()) return;
      connections.value = next;
      clearError();
    } catch (failure) {
      if (ticket.isCurrent()) error.value = capture(failure);
    } finally {
      if (ticket.isCurrent()) loading.value = false;
    }
  }

  /**
   * Creates a webhook connection. The label must already satisfy `isValidLabel`. The
   * result is `false` when the secret was not published, either because the request
   * failed or because the panel closed or the account changed first.
   */
  async function connect(label: string): Promise<boolean> {
    const secret = secretScope.current();
    const view = operationScope.current();
    const leave = activity.enter();
    clearError();
    try {
      const created = await connectWebhook(client, label.trim());
      if (!secret.isCurrent()) return false;
      issued.value = created;
      await refresh();
      return true;
    } catch (failure) {
      if (view.isCurrent()) error.value = capture(failure);
      return false;
    } finally {
      leave();
    }
  }

  function dismissSecret(): void {
    issued.value = null;
  }

  async function openEvents(id: string): Promise<void> {
    const ticket = eventScope.begin();
    selectedId.value = id;
    events.value = null;
    eventsError.value = null;
    try {
      const next = await readEvents(client, id);
      if (ticket.isCurrent()) events.value = next;
    } catch (failure) {
      if (ticket.isCurrent()) eventsError.value = errorMessage(failure);
    }
  }

  function closeEvents(): void {
    eventScope.invalidate();
    selectedId.value = null;
    events.value = null;
    eventsError.value = null;
  }

  /** Revokes a connection. Later deliveries are refused; stored events stay readable. */
  async function remove(id: string): Promise<boolean> {
    const view = operationScope.current();
    const leave = activity.enter();
    clearError();
    try {
      await disconnect(client, id);
      // The revocation happened whatever the view is now; only the view is left alone.
      if (!view.isCurrent()) return true;
      if (selectedId.value === id) closeEvents();
      await refresh();
      return true;
    } catch (failure) {
      if (view.isCurrent()) error.value = capture(failure);
      return false;
    } finally {
      leave();
    }
  }

  /**
   * Drops the secret and any open event view, for example when the panel closes. Replies
   * that are still on their way no longer touch the state, and the activity starts again.
   */
  function reset(): void {
    secretScope.invalidate();
    listScope.invalidate();
    operationScope.invalidate();
    activity.reset();
    loading.value = false;
    shown = false;
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
