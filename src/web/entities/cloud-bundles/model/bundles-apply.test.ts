import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createJournal, parseJournal, serializeJournal, setEntry } from './apply-journal.ts';
import { JOURNAL_KEY, useBundlesStore } from './bundles-store.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

function host(preflight: unknown, stored: Record<string, string> = {}) {
  const writes: Array<{ key: string; value: string }> = [];
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'rpc.discover') return { methods: [{ name: 'bundles.packages.preflight' }] };
    if (method === 'bundles.packages.preflight') return preflight;
    if (method === 'app.state.set') {
      const write = params as { key: string; value: string };
      writes.push(write);
      stored[write.key] = write.value;
      return { state: {} };
    }
    if (method === 'app.state.get') return { state: stored };
    throw new Error(`unexpected ${method}`);
  });
  return { stores: createStoreHarness(client), calls, writes, stored };
}

const report = (items: unknown[], canApply: boolean) => ({ packageVersion: 1, items, blocking: [], canApply, applyMode: 'clientJournal' });

describe('package apply', () => {
  it('refuses to apply a package whose last check did not allow it', async () => {
    const { stores, writes } = host(report([{ kind: 'overlay', id, version: null, revision: 2, required: true, status: 'revisionChanged', permissions: 'none' }], false));
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(id);
    expect(await bundles.applyPackage()).toBe(false);
    expect(writes).toHaveLength(0);
  });

  it('records skipped overlays and profiles honestly and persists every step', async () => {
    const { stores, writes } = host(report([
      { kind: 'overlay', id, version: null, revision: 2, required: true, status: 'ready', permissions: 'none' },
      { kind: 'profile', id, version: null, revision: 1, required: false, status: 'ready', permissions: 'none' },
    ], true));
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(id);
    expect(await bundles.applyPackage()).toBe(true);
    const journal = bundles.journal;
    expect(journal?.entries.map((entry) => entry.status)).toEqual(['skipped', 'skipped']);
    expect(journal?.entries[0]?.message).toMatch(/overlays cannot be applied/);
    expect(writes.length).toBeGreaterThanOrEqual(4);
    expect(writes.every((write) => write.key === JOURNAL_KEY)).toBe(true);
  });

  it('fails a template step without a signed-in account and keeps the failure visible', async () => {
    const { stores } = host(report([{ kind: 'template', id, version: null, revision: 4, required: true, status: 'ready', permissions: 'none' }], true));
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(id);
    await bundles.applyPackage();
    expect(bundles.journal?.entries[0]).toMatchObject({ status: 'failed', message: 'sign in to Cloud to import this template' });
  });

  it('recovers a step that was running when the app stopped without retrying it', async () => {
    const saved = setEntry(createJournal(id, [{ kind: 'template', id, version: null, revision: 4, status: 'ready' }], 1), 0, 'running');
    const { stores, writes } = host(report([], true), { [JOURNAL_KEY]: serializeJournal(saved) });
    const bundles = stores.use(() => useBundlesStore());
    await bundles.loadJournal();
    expect(bundles.journal?.entries[0]?.status).toBe('interrupted');
    expect(parseJournal(writes.at(-1)!.value)?.entries[0]?.status).toBe('interrupted');
  });
});
