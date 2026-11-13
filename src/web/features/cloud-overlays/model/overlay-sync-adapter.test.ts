import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useCloudOverlayStore } from '@/entities/overlay/model/cloud-overlay-store.ts';
import { emptyOverlay } from '../../../../shared/cloud-overlay-schema.ts';
import { createOverlaySyncAdapter } from './overlay-sync-adapter.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('overlay revision Changes adapter', () => {
  it('sends exactly the reviewed draft through the existing create/revise API', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'bundles.overlays.create') return { id, revision: 1, status: 'draft' };
      if (method === 'bundles.overlays.revise') return { id, revision: 2, status: 'draft' };
      throw new Error(method);
    });
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    const adapter = createOverlaySyncAdapter({ overlay, supports: () => true, t: (key) => key });
    expect(adapter.requiresSyncOptIn).toBe(false);
    expect(adapter.capabilities).toMatchObject({ push: true, fetch: true, apply: true, force: false, merge: false, autoSync: false });
    overlay.startNew('chat', 'First revision');
    await adapter.push!((await adapter.list()).map((row) => row.id));
    overlay.setTitle('Second revision');
    await adapter.push!((await adapter.list()).map((row) => row.id));
    expect(calls.map((call) => call.method)).toEqual(['bundles.overlays.create', 'bundles.overlays.revise']);
    expect(calls[1]?.params).toMatchObject({ id, expectedRevision: 1, document: { title: 'Second revision' } });
    expect(await adapter.list()).toEqual([]);
  });

  it('fetches without applying and permits only reviewed Cloud adoption after a conflict', async () => {
    let revision = 1;
    const { client, calls } = createFakeControlClient(() => ({ id, revision, status: 'draft', document: emptyOverlay('chat', `Revision ${revision}`) }));
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    const adapter = createOverlaySyncAdapter({ overlay, supports: () => true, t: (key) => key });
    await overlay.load(id);
    overlay.setTitle('Local draft');
    revision = 2;
    await adapter.fetch!();
    expect(overlay.draft.title).toBe('Local draft');
    const [row] = await adapter.list();
    expect(row).toMatchObject({ side: 'conflict', canSend: false, canResolveLocal: false, canResolveCloud: true });
    await expect(adapter.push!([row!.id])).rejects.toThrow('syncChanges.overlayCloudChanged');
    await expect(adapter.resolve!(row!.id, { choice: 'local' })).rejects.toThrow('syncChanges.overlayCloudChanged');
    await adapter.resolve!(row!.id, { choice: 'cloud' });
    expect(overlay.draft.title).toBe('Revision 2');
    expect(calls.map((call) => call.method)).toEqual(['bundles.overlays.read', 'bundles.overlays.read']);
  });

  it('restores a local edit from the reviewed cached revision without sending or fetching', async () => {
    const { client, calls } = createFakeControlClient(() => ({ id, revision: 1, status: 'draft', document: emptyOverlay('chat', 'Saved') }));
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    const adapter = createOverlaySyncAdapter({ overlay, supports: () => true, t: (key) => key });
    await overlay.load(id);
    overlay.setTitle('Unsaved');
    const [row] = await adapter.list();
    expect(row).toMatchObject({ side: 'local', canApply: true });
    await adapter.apply!(row!.id);
    expect(overlay.draft.title).toBe('Saved');
    expect(calls).toHaveLength(1);
    expect(await adapter.list()).toEqual([]);
  });
});
