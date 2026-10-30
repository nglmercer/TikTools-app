import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { normalizeDesign, parseDesign } from '../../../../widgets/sdk/design.ts';
import type { WidgetStyle } from '../../../../widgets/sdk/template.ts';
import type { WidgetKind } from '../../../../widgets/sdk/templates.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { overlayKinds, parseGatewayStatus, type GatewayStatus } from '../lib/overlay-host.ts';

export const STARTING_RETRIES = 5;
export const STARTING_RETRY_MS = 2_000;

export interface CopyOutcome {
  ok: boolean;
  message: string;
}

const designKey = (kind: WidgetKind): string => `widgets.design.${kind}`;

export const useOverlaysStore = defineStore('overlays', () => {
  const client = useControlClient();
  const status = shallowRef<GatewayStatus | null>(null);
  const statusError = ref<string | null>(null);
  const refreshing = ref(false);
  const designs = shallowRef<Partial<Record<WidgetKind, WidgetStyle>>>({});
  const designsLoading = ref(false);
  const designsError = ref<string | null>(null);
  let startingAttempts = 0;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;

  function clearRetry(): void {
    if (retryTimer !== null) clearTimeout(retryTimer);
    retryTimer = null;
  }

  async function probe(): Promise<void> {
    clearRetry();
    refreshing.value = true;
    try {
      status.value = parseGatewayStatus(await client.call<unknown>('widgets.status', {}));
      statusError.value = null;
    } catch (failure) {
      statusError.value = errorMessage(failure);
    } finally {
      refreshing.value = false;
    }
    if (status.value?.state === 'starting' && startingAttempts < STARTING_RETRIES) {
      startingAttempts += 1;
      retryTimer = setTimeout(() => void probe(), STARTING_RETRY_MS);
    }
  }

  function refresh(): Promise<void> {
    startingAttempts = 0;
    return probe();
  }

  /** Ranking widgets also need their ranking and visualization options. */
  async function copySourceUrl(kind: WidgetKind, options: Record<string, unknown> = {}): Promise<CopyOutcome> {
    try {
      const result = await client.call<{ ok?: unknown }>('widgets.copyObsUrl', { ...options, widget: kind });
      return { ok: result?.ok === true, message: '' };
    } catch (failure) {
      return { ok: false, message: errorMessage(failure) };
    }
  }

  async function loadDesigns(): Promise<void> {
    designsLoading.value = true;
    designsError.value = null;
    try {
      const result = await client.call<{ state: Record<string, string> }>('app.state.get', {
        keys: overlayKinds.map(designKey),
      });
      designs.value = Object.fromEntries(overlayKinds.map((kind) => [kind, parseDesign(result.state[designKey(kind)] ?? '{}')]));
    } catch (failure) {
      designsError.value = errorMessage(failure);
    } finally {
      designsLoading.value = false;
    }
  }

  async function saveDesign(kind: WidgetKind, design: WidgetStyle): Promise<void> {
    const normalized = normalizeDesign(design);
    await client.call('app.state.set', { key: designKey(kind), value: JSON.stringify(normalized) });
    designs.value = { ...designs.value, [kind]: normalized };
  }

  return {
    status,
    statusError,
    refreshing,
    designs,
    designsLoading,
    designsError,
    refresh,
    copySourceUrl,
    loadDesigns,
    saveDesign,
    dispose: clearRetry,
  };
});
