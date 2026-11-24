import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createApp } from 'vue';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { STARTING_RETRIES, STARTING_RETRY_MS, useOverlaysStore } from '../../../../../../src/web/entities/overlay/model/overlays-store.ts';

function setup(handler: (method: string) => unknown) {
  const { client, calls } = createFakeControlClient((method) => handler(method));
  const app = createApp({ render: () => null });
  const pinia = createPinia();
  app.use(pinia);
  app.provide(controlClientKey, client);
  setActivePinia(pinia);
  const store = app.runWithContext(() => useOverlaysStore());
  return { store, calls };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('overlays store', () => {
  it('stores the parsed gateway status', async () => {
    const { store, calls } = setup(() => ({ state: 'ready', port: 19999, error: null }));
    await store.refresh();
    expect(calls).toEqual([{ method: 'widgets.status', params: {} }]);
    expect(store.status).toEqual({ state: 'ready', port: 19999, error: null });
    expect(store.refreshing).toBe(false);
  });

  it('keeps the last status and reports failures', async () => {
    const { store } = setup(() => {
      throw new ControlCallError('x', 'boom');
    });
    await store.refresh();
    expect(store.status).toBeNull();
    expect(store.statusError).toBe('boom');
  });

  it('re-probes a starting gateway a bounded number of times', async () => {
    const { store, calls } = setup(() => ({ state: 'starting', port: 17452 }));
    await store.refresh();
    for (let attempt = 0; attempt < STARTING_RETRIES + 2; attempt += 1) await vi.advanceTimersByTimeAsync(STARTING_RETRY_MS);
    expect(calls).toHaveLength(STARTING_RETRIES + 1);
    store.dispose();
  });

  it('delegates copying the source url to the host', async () => {
    const { store, calls } = setup((method) => {
      if (method === 'widgets.copyObsUrl') return { ok: true };
      return {};
    });
    await expect(store.copySourceUrl('gift')).resolves.toEqual({ ok: true, message: '' });
    expect(calls).toEqual([{ method: 'widgets.copyObsUrl', params: { widget: 'gift' } }]);
  });

  it('reports copy failures without token material', async () => {
    const { store } = setup(() => {
      throw new ControlCallError('unavailable', 'no widget credential is stored');
    });
    await expect(store.copySourceUrl('follow')).resolves.toEqual({ ok: false, message: 'no widget credential is stored' });
  });

  it('loads designs and only updates them after a successful save', async () => {
    let fail = false;
    const { store, calls } = setup((method) => {
      if (method === 'app.state.get') return { state: { 'widgets.design.follow': '{"radius":24}', 'widgets.design.chat': 'broken' } };
      if (fail) throw new Error('storage unavailable');
      return { ok: true };
    });
    await store.loadDesigns();
    expect(store.designs.follow).toEqual({ radius: 24 });
    expect(store.designs.chat).toEqual({});
    await store.saveDesign('follow', { accent: '#112233', radius: 99 });
    expect(calls.at(-1)).toEqual({
      method: 'app.state.set',
      params: { key: 'widgets.design.follow', value: '{"accent":"#112233","radius":48}' },
    });
    fail = true;
    await expect(store.saveDesign('follow', {})).rejects.toThrow('storage unavailable');
    expect(store.designs.follow).toEqual({ accent: '#112233', radius: 48 });
  });
});
