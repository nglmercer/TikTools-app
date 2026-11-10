import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useOverlayStore } from './overlay-store.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('overlay store', () => {
  it('refuses to save a document the schema rejects, without a request', async () => {
    const { client, calls } = createFakeControlClient(() => { throw new Error('unexpected'); });
    const overlay = createStoreHarness(client).use(() => useOverlayStore());
    overlay.startNew('gift', '');
    expect(await overlay.save()).toBe(false);
    expect(calls).toHaveLength(0);
  });

  it('creates a new overlay, then revises it with the revision it holds', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'bundles.overlays.create') return { id, revision: 1, status: 'draft', schemaVersion: 1 };
      if (method === 'bundles.overlays.revise') return { id, revision: 2, status: 'draft' };
      throw new Error(method);
    });
    const overlay = createStoreHarness(client).use(() => useOverlayStore());
    overlay.startNew('chat', 'Chat');
    expect(await overlay.save()).toBe(true);
    expect(overlay.revision).toBe(1);
    overlay.add('text');
    expect(await overlay.save()).toBe(true);
    expect(calls.at(-1)?.method).toBe('bundles.overlays.revise');
    expect(calls.at(-1)?.params).toMatchObject({ id, expectedRevision: 1 });
    expect(overlay.revision).toBe(2);
  });

  it('declares the live-read capability on the saved document when an avatar reads the stream', async () => {
    const { client, calls } = createFakeControlClient(() => ({ id, revision: 1, status: 'draft', schemaVersion: 1 }));
    const overlay = createStoreHarness(client).use(() => useOverlayStore());
    overlay.startNew('follow', 'Follow');
    overlay.add('avatar');
    expect(await overlay.save()).toBe(true);
    expect((calls[0]?.params as { document: { capabilities?: string[] } }).document.capabilities).toEqual(['live.read']);
  });
});
