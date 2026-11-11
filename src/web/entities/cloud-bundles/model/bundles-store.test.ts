import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { MAX_IMAGE_BYTES, useBundlesStore } from './bundles-store.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

function host(handler: (method: string, params: unknown) => unknown) {
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'rpc.discover') return { methods: [{ name: 'bundles.packages.preflight' }, { name: 'bundles.assets.upload' }] };
    return handler(method, params);
  });
  return { stores: createStoreHarness(client), calls };
}

describe('bundles store', () => {
  it('refuses an oversized image before any request', async () => {
    const { stores, calls } = host(() => ({}));
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    const big = new File([new Uint8Array(MAX_IMAGE_BYTES + 1)], 'big.png', { type: 'image/png' });
    expect(await bundles.upload(big)).toBe(false);
    expect(bundles.error).toBe('size');
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover']);
  });

  it('uploads base64 bytes and keeps the returned asset', async () => {
    const { stores, calls } = host((method) => {
      if (method === 'bundles.assets.upload') return { id, mime: 'image/png', width: 2, height: 2, size: 4 };
      throw new Error(method);
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    expect(await bundles.upload(new File([new Uint8Array([1, 2, 3, 4])], 'a.png', { type: 'image/png' }))).toBe(true);
    expect(bundles.asset?.id).toBe(id);
    expect(calls.at(-1)?.params).toEqual({ dataBase64: btoa('\u0001\u0002\u0003\u0004') });
  });

  it('shows the preflight for the requested package and never applies it', async () => {
    const { stores, calls } = host((method) => {
      if (method === 'bundles.packages.preflight') {
        return { items: [{ kind: 'overlay', id, version: null, revision: 2, required: true, status: 'revisionChanged', permissions: 'none' }], blocking: [{}], canApply: false, applyMode: 'clientJournal' };
      }
      throw new Error(method);
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(id);
    expect(bundles.preflight?.canApply).toBe(false);
    expect(bundles.preflight?.items[0]?.status).toBe('revisionChanged');
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover', 'bundles.packages.preflight']);
  });
});
