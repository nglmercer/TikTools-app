/**
 * Widgets host bridge: the WebView never handles raw gateway secrets.
 *
 * Plugin settings arrive redacted, so this module never reads the OBS
 * credential from settings state and never probes the gateway over
 * fetch (browser origins/CORS differ per shell). All credential and
 * reachability questions go to the host (`widgets.status`,
 * `widgets.copyObsUrl`); the UI only renders the returned state and a
 * redacted URL placeholder.
 */

import { ref } from 'vue';
import type { WidgetStyle } from '../../widgets/sdk/template.ts';
import { normalizeDesign, parseDesign } from '../../widgets/sdk/design.ts';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';

export const GATEWAY_PLUGIN_ID = 'tiktools.event-gateway';
export const GATEWAY_DEFAULT_HOST = '127.0.0.1';
export const GATEWAY_DEFAULT_PORT = 17452;

/** Redacted marker shown in place of the real URL credential. */
export const REDACTED_TOKEN = '••••••••';

export type WidgetKind = 'follow' | 'gift' | 'chat' | 'share' | 'subscribe';

export type WidgetsHostState =
  | 'missing'
  | 'disabled'
  | 'stopped'
  | 'starting'
  | 'credential-unavailable'
  | 'assets-missing'
  | 'unreachable'
  | 'ready';

export interface WidgetsStatus {
  state: WidgetsHostState;
  port: number;
  error: string | null;
}

export interface WidgetsCopyFeedback {
  widget: WidgetKind;
  ok: boolean;
  message: string;
}

const WIDGET_STATES: readonly WidgetsHostState[] = [
  'missing',
  'disabled',
  'stopped',
  'starting',
  'credential-unavailable',
  'assets-missing',
  'unreachable',
  'ready',
];

/** Display-only OBS URL: the credential slot always shows bullets. */
export function buildRedactedObsUrl(port: number, widget: WidgetKind): string {
  return `http://${GATEWAY_DEFAULT_HOST}:${port}/widgets/${widget}/#token=${REDACTED_TOKEN}`;
}

/** Tokenless demo preview: the widget plays synthetic content locally. */
export function buildWidgetPreviewUrl(port: number, widget: WidgetKind): string {
  const demo = widget === 'follow' ? 'follow' : widget === 'gift' ? 'combo' : widget;
  return `http://${GATEWAY_DEFAULT_HOST}:${port}/widgets/${widget}/#demo=${demo}`;
}

/**
 * Preview iframes load the bundles from the gateway itself: mounting one
 * against a stopped/missing gateway renders a blank frame, so previews
 * only mount when the host reports `ready`.
 */
export function canPreviewWidgets(state: WidgetsHostState | null): boolean {
  return state === 'ready';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Defensively parses a `widgets.status` result; garbage becomes `unreachable`. */
export function parseWidgetsStatus(value: unknown): WidgetsStatus {
  if (!isRecord(value)) {
    return { state: 'unreachable', port: GATEWAY_DEFAULT_PORT, error: null };
  }
  const state = WIDGET_STATES.includes(value['state'] as WidgetsHostState)
    ? (value['state'] as WidgetsHostState)
    : 'unreachable';
  const port =
    typeof value['port'] === 'number' &&
    Number.isInteger(value['port']) &&
    (value['port'] as number) >= 1 &&
    (value['port'] as number) <= 65535
      ? (value['port'] as number)
      : GATEWAY_DEFAULT_PORT;
  const error =
    typeof value['error'] === 'string' && (value['error'] as string).trim() !== ''
      ? (value['error'] as string)
      : null;
  return { state, port, error };
}

export function useWidgets(control: ControlClient) {
  const designs = ref<Partial<Record<WidgetKind, WidgetStyle>>>({});
  const designsLoading = ref(false);
  const designsError = ref<string | null>(null);
  const kinds: WidgetKind[] = ['follow', 'gift', 'chat', 'share', 'subscribe'];
  const loadDesigns = async (): Promise<void> => {
    designsLoading.value = true;
    designsError.value = null;
    try {
      const result = await control.call<{ state: Record<string, string> }>('app.state.get', {
        keys: kinds.map((kind) => `widgets.design.${kind}`),
      });
      for (const kind of kinds) designs.value[kind] = parseDesign(result.state[`widgets.design.${kind}`] ?? '{}');
    } catch (failure) {
      designsError.value = errorMessage(failure);
    } finally { designsLoading.value = false; }
  };
  const saveDesign = async (kind: WidgetKind, design: WidgetStyle): Promise<void> => {
    const normalized = normalizeDesign(design);
    await control.call('app.state.set', { key: `widgets.design.${kind}`, value: JSON.stringify(normalized) });
    designs.value[kind] = normalized;
  };
  const status = ref<WidgetsStatus | null>(null);
  const statusError = ref<string | null>(null);
  const refreshing = ref(false);
  const lastCopy = ref<WidgetsCopyFeedback | null>(null);

  const refreshStatus = (onDone?: (next: WidgetsStatus | null) => void): void => {
    refreshing.value = true;
    void control
      .call<WidgetsStatus>('widgets.status', {})
      .then((result) => {
        status.value = parseWidgetsStatus(result);
        statusError.value = null;
        refreshing.value = false;
        onDone?.(status.value);
      })
      .catch((failure: unknown) => {
        statusError.value = errorMessage(failure);
        refreshing.value = false;
        onDone?.(null);
      });
  };

  const copyObsUrl = (
    widget: WidgetKind,
    onDone: (feedback: WidgetsCopyFeedback) => void,
  ): void => {
    void control
      .call<{ ok?: unknown }>('widgets.copyObsUrl', { widget })
      .then((result) => {
        const feedback: WidgetsCopyFeedback = {
          widget,
          ok: isRecord(result) && result['ok'] === true,
          message: '',
        };
        lastCopy.value = feedback;
        onDone(feedback);
      })
      .catch((failure: unknown) => {
        const feedback: WidgetsCopyFeedback = {
          widget,
          ok: false,
          message: errorMessage(failure),
        };
        lastCopy.value = feedback;
        onDone(feedback);
      });
  };

  return {
    designs, designsLoading, designsError, loadDesigns, saveDesign,
    status,
    statusError,
    refreshing,
    lastCopy,
    refreshStatus,
    copyObsUrl,
  };
}
