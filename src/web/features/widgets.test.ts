import { describe, expect, test } from 'bun:test';

import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError } from '../platform/control-client.ts';
import {
  buildRedactedObsUrl,
  buildWidgetPreviewUrl,
  GATEWAY_DEFAULT_PORT,
  parseWidgetsStatus,
  REDACTED_TOKEN,
  useWidgets,
  type WidgetsCopyFeedback,
} from './widgets.ts';

function fakeControl(
  handler: (method: string) => Promise<unknown>,
): { client: ControlClient; calls: Array<{ method: string; params: unknown }> } {
  const calls: Array<{ method: string; params: unknown }> = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: ((method: string, params?: unknown) => {
      calls.push({ method, params });
      return handler(method);
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { client, calls };
}

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

describe('widgets URL builders', () => {
  test('redacted OBS URL never carries credential material', () => {
    expect(buildRedactedObsUrl(17452, 'follow')).toBe(
      `http://127.0.0.1:17452/widgets/follow/#token=${REDACTED_TOKEN}`,
    );
    expect(buildRedactedObsUrl(19999, 'gift')).toBe(
      'http://127.0.0.1:19999/widgets/gift/#token=••••••••',
    );
    expect(buildRedactedObsUrl(17452, 'follow')).not.toContain('?token=');
  });

  test('preview URLs stay in tokenless demo mode', () => {
    expect(buildWidgetPreviewUrl(17452, 'follow')).toBe(
      'http://127.0.0.1:17452/widgets/follow/#demo=follow',
    );
    expect(buildWidgetPreviewUrl(17452, 'gift')).toBe(
      'http://127.0.0.1:17452/widgets/gift/#demo=combo',
    );
  });
});

describe('parseWidgetsStatus', () => {
  test('passes valid host payloads through', () => {
    expect(parseWidgetsStatus({ state: 'ready', port: 19999, error: null })).toEqual({
      state: 'ready',
      port: 19999,
      error: null,
    });
    expect(parseWidgetsStatus({ state: 'starting', port: 17452, error: 'retry' })).toEqual({
      state: 'starting',
      port: 17452,
      error: 'retry',
    });
  });

  test('coerces garbage into unreachable with safe defaults', () => {
    expect(parseWidgetsStatus(null)).toEqual({
      state: 'unreachable',
      port: GATEWAY_DEFAULT_PORT,
      error: null,
    });
    expect(parseWidgetsStatus({ state: 'melted', port: 0, error: '  ' })).toEqual({
      state: 'unreachable',
      port: GATEWAY_DEFAULT_PORT,
      error: null,
    });
    expect(parseWidgetsStatus({ state: 'ready', port: 99999 })).toEqual({
      state: 'ready',
      port: GATEWAY_DEFAULT_PORT,
      error: null,
    });
  });
});

describe('useWidgets host bridge', () => {
  test('refreshStatus stores host state and clears errors', async () => {
    const { client, calls } = fakeControl(() =>
      Promise.resolve({ state: 'ready', port: 19999, error: null }),
    );
    const widgets = useWidgets(client);
    widgets.refreshStatus();
    expect(widgets.refreshing.value).toBe(true);
    await flush();
    expect(calls).toEqual([{ method: 'widgets.status', params: {} }]);
    expect(widgets.refreshing.value).toBe(false);
    expect(widgets.status.value).toEqual({ state: 'ready', port: 19999, error: null });
    expect(widgets.statusError.value).toBeNull();
  });

  test('refreshStatus failure surfaces a message and keeps last state', async () => {
    const { client } = fakeControl(() => Promise.reject(new ControlCallError('x', 'boom')));
    const widgets = useWidgets(client);
    widgets.refreshStatus();
    await flush();
    expect(widgets.refreshing.value).toBe(false);
    expect(widgets.status.value).toBeNull();
    expect(widgets.statusError.value).toBe('boom');
  });

  test('copyObsUrl delegates the URL to the host clipboard op', async () => {
    const { client, calls } = fakeControl(() => Promise.resolve({ ok: true }));
    const widgets = useWidgets(client);
    const seen: WidgetsCopyFeedback[] = [];
    widgets.copyObsUrl('gift', (next) => {
      seen.push(next);
    });
    await flush();
    expect(calls).toEqual([{ method: 'widgets.copyObsUrl', params: { widget: 'gift' } }]);
    expect(seen).toEqual([{ widget: 'gift', ok: true, message: '' }]);
    expect(widgets.lastCopy.value).toEqual({ widget: 'gift', ok: true, message: '' });
  });

  test('copyObsUrl failure reports the host message without token material', async () => {
    const { client } = fakeControl(() =>
      Promise.reject(new ControlCallError('unavailable', 'no widget credential is stored')),
    );
    const widgets = useWidgets(client);
    const seen: WidgetsCopyFeedback[] = [];
    widgets.copyObsUrl('follow', (next) => {
      seen.push(next);
    });
    await flush();
    expect(seen).toEqual([
      { widget: 'follow', ok: false, message: 'no widget credential is stored' },
    ]);
    expect([widgets.lastCopy.value]).toEqual(seen);
  });
});
