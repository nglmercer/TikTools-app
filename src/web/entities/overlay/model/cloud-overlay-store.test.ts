import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useCloudOverlayStore } from './cloud-overlay-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { emptyOverlay } from '../../../../shared/cloud-overlay-schema.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const record = (revision = 1, title = 'Cloud') => ({ id, revision, status: 'draft', document: emptyOverlay('chat', title) });
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe('overlay store', () => {
  it('refuses to save a document the schema rejects, without a request', async () => {
    const { client, calls } = createFakeControlClient(() => { throw new Error('unexpected'); });
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
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
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
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
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    overlay.startNew('follow', 'Follow');
    overlay.add('avatar');
    expect(await overlay.save()).toBe(true);
    expect((calls[0]?.params as { document: { capabilities?: string[] } }).document.capabilities).toEqual(['live.read']);
  });

  it('copies the draft as an OBS URL payload and refuses one that does not fit', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'widgets.copyCloudOverlayUrl') return { ok: true };
      throw new Error(method);
    });
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    overlay.startNew('chat', 'Chat');
    overlay.add('text');
    expect(await overlay.copyObsUrl()).toBe(true);
    expect(overlay.copied).toBe(true);
    expect(calls.at(-1)?.method).toBe('widgets.copyCloudOverlayUrl');
    const before = calls.length;
    overlay.add('text');
    overlay.patch(overlay.draft.elements[1]!.id, { width: 0 });
    expect(await overlay.copyObsUrl()).toBe(false);
    expect(calls.length).toBe(before);
  });

  it('fetches a comparison without replacing the draft and adopts it without a write', async () => {
    let remote = record();
    const { client, calls } = createFakeControlClient(() => remote);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    expect(await overlay.load(id)).toBe(true);
    overlay.setTitle('Local');
    remote = record(2, 'Remote');
    expect(await overlay.fetchRevision()).toBe(true);
    expect(overlay.draft.title).toBe('Local');
    expect(overlay.revision).toBe(1);
    const review = overlay.review()!;
    expect(review).toMatchObject({ conflict: true, canSend: false, local: { title: 'Local' }, base: { title: 'Cloud' }, cloud: { title: 'Remote' } });
    const before = calls.length;
    overlay.applyReview(review.id);
    expect(overlay.draft.title).toBe('Remote');
    expect(overlay.revision).toBe(2);
    expect(overlay.review()).toBeNull();
    expect(calls).toHaveLength(before);
  });

  it('rejects an old comparison after an edit without sending any document', async () => {
    const { client, calls } = createFakeControlClient(() => record());
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    await overlay.load(id);
    overlay.setTitle('First edit');
    const old = overlay.review()!;
    expect(overlay.review()?.id).toBe(old.id);
    overlay.setTitle('Second edit');
    await expect(overlay.saveReview(old.id)).rejects.toThrow('overlay-review-changed');
    expect(() => overlay.applyReview(old.id)).toThrow('overlay-review-changed');
    expect(calls).toHaveLength(1);
  });

  it('keeps newer edits while acknowledging only the document submitted by a save', async () => {
    const pending = deferred<{ id: string; revision: number; status: string }>();
    const { client, calls } = createFakeControlClient(() => pending.promise);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    overlay.startNew('chat', 'Submitted');
    const saving = overlay.save();
    overlay.setTitle('Newer draft');
    pending.resolve({ id, revision: 1, status: 'draft' });
    expect(await saving).toBe(true);
    expect(calls[0]?.params).toMatchObject({ document: { title: 'Submitted' } });
    expect(overlay.draft.title).toBe('Newer draft');
    expect(overlay.review()).toMatchObject({ base: { title: 'Submitted' }, local: { title: 'Newer draft' }, canSend: true });
  });

  it('does not replace a draft edited while Open was waiting for Cloud', async () => {
    const pending = deferred<ReturnType<typeof record>>();
    const { client } = createFakeControlClient(() => pending.promise);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    const opening = overlay.load(id);
    overlay.setTitle('Edited while opening');
    pending.resolve(record());
    expect(await opening).toBe(false);
    expect(overlay.draft.title).toBe('Edited while opening');
    expect(overlay.overlayId).toBeNull();
    expect(overlay.loading).toBe(false);
    expect(overlay.error).toBe('overlay-draft-changed');
  });

  it('drops an in-flight save after a new local document is started', async () => {
    const pending = deferred<{ id: string; revision: number; status: string }>();
    const { client } = createFakeControlClient(() => pending.promise);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    const saving = overlay.save();
    overlay.startNew('gift', 'Another document');
    pending.resolve({ id, revision: 1, status: 'draft' });
    expect(await saving).toBe(false);
    expect(overlay.overlayId).toBeNull();
    expect(overlay.draft.title).toBe('Another document');
    expect(overlay.saving).toBe(false);
  });

  it('invalidates even a new local document review on an account transition', async () => {
    const { client, calls } = createFakeControlClient(() => { throw new Error('unexpected'); });
    const harness = createStoreHarness(client);
    const account = harness.use(() => useAccountStore());
    const overlay = harness.use(() => useCloudOverlayStore());
    const old = overlay.review()!;
    account.working = true;
    await expect(overlay.saveReview(old.id)).rejects.toThrow('overlay-review-changed');
    expect(calls).toHaveLength(0);
  });

  it('clears bound account data and drops a late download during logout', async () => {
    const pending = deferred<ReturnType<typeof record>>();
    let reads = 0;
    const { client } = createFakeControlClient(() => ++reads === 1 ? record() : pending.promise);
    const harness = createStoreHarness(client);
    const account = harness.use(() => useAccountStore());
    const overlay = harness.use(() => useCloudOverlayStore());
    await overlay.load(id);
    const fetching = overlay.fetchRevision();
    account.working = true;
    pending.resolve(record(2, 'Old account data'));
    expect(await fetching).toBe(false);
    expect(overlay.overlayId).toBeNull();
    expect(overlay.draft.title).toBe('New overlay');
    expect(overlay.loading).toBe(false);
  });

  it('reviews a newer revision even when its document has the same content', async () => {
    let remote = record();
    const { client } = createFakeControlClient(() => remote);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    await overlay.load(id);
    remote = record(2);
    await overlay.fetchRevision();
    const review = overlay.review()!;
    expect(review).not.toBeNull();
    overlay.applyReview(review.id);
    expect(overlay.revision).toBe(2);
  });

  it('rejects malformed Cloud documents before exposing them as drafts', async () => {
    const { client } = createFakeControlClient(() => ({ ...record(), document: { title: 'Incomplete' } }));
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    expect(await overlay.load(id)).toBe(false);
    expect(overlay.overlayId).toBeNull();
    expect(overlay.draft.title).toBe('New overlay');
  });

  it.each([
    { ...emptyOverlay('chat', 'Invalid'), overlayVersion: 2 },
    { ...emptyOverlay('chat', 'Invalid'), kind: 'unknown' },
    { ...emptyOverlay('chat', 'Invalid'), elements: [{ id: 'one', type: 'script', x: 0, y: 0, width: 1, height: 1 }] },
    { ...emptyOverlay('chat', 'Invalid'), elements: [{ id: 'one', type: 'avatar', x: 0, y: 0, width: 1, height: 1, opacity: null }] },
    { ...emptyOverlay('chat', 'Invalid'), elements: [{ id: 'one', type: 'text', x: 0, y: 0, width: 1, height: 1, source: { kind: 'other', field: 'viewer.name' } }] },
    { ...emptyOverlay('chat', 'Invalid'), capabilities: ['shell.execute'] },
  ])('rejects unsupported Cloud schema shapes %#', async (document) => {
    const { client } = createFakeControlClient(() => ({ ...record(), document }));
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    expect(await overlay.load(id)).toBe(false);
    expect(overlay.overlayId).toBeNull();
  });

  it('compares documents by value when Cloud object keys arrive in another order', async () => {
    const remote = record();
    remote.document.canvas = { height: 1080, width: 1920 };
    const { client } = createFakeControlClient(() => remote);
    const overlay = createStoreHarness(client).use(() => useCloudOverlayStore());
    await overlay.load(id);
    overlay.setCanvas(1920, 1080);
    expect(overlay.review()).toBeNull();
  });
});
