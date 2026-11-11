import { describe, expect, it, vi } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
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

  it('checks overlays and profiles against their pinned revisions and persists every step', async () => {
    const { stores, writes } = host(report([
      { kind: 'overlay', id, version: null, revision: 2, required: true, status: 'ready', permissions: 'none' },
      { kind: 'profile', id, version: null, revision: 1, required: false, status: 'ready', permissions: 'none' },
    ], true));
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(id);
    expect(await bundles.applyPackage()).toBe(true);
    const journal = bundles.journal;
    expect(journal?.entries.map((entry) => entry.status)).toEqual(['failed', 'failed']);
    expect(journal?.entries[1]?.message).toBe('sign in to Cloud to import this profile');
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

describe('package apply with Cloud templates and profiles', () => {
  const documentId = '00000000-0000-4000-8000-000000000001';
  const template = { templateVersion: 1, id: 'bundle-template', title: 'Bundle template', description: '', icon: 'gift',
    actions: [{ name: 'act', typeId: 'core.log', config: {} }], event: { name: 'evt', trigger: 'tiktok.chat' } };

  async function ownerHost() {
    const items = [{ kind: 'template', id: documentId, version: null, revision: 4, required: true, status: 'ready', permissions: 'none' }];
    const native = nativeDefinitionHost((method, params, options) => {
      if (method === 'rpc.discover') return { methods: [{ name: 'bundles.packages.preflight' }] };
      if (method === 'bundles.packages.preflight') return report(items, true);
      if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
      if (method === 'app.state.get') return { state: {} };
      if (method === 'app.state.set') return { state: {} };
      if (method === 'cloudLibrary.read') return { meta: { id: documentId, revision: '4', sha256: 'a'.repeat(64) }, document: template };
      if (method === 'automation.create') {
        options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: (params?.record as { id: string }).id, revision: '1' }] });
        return params?.record;
      }
      throw new Error(`unexpected ${method}`);
    });
    const { client, calls } = createFakeControlClient(native.handle);
    const stores = createStoreHarness(client);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    const bundles = stores.use(() => useBundlesStore());
    await bundles.checkPackage(documentId);
    return { bundles, calls, journal: libraryJournal(client) };
  }

  it('records a confirmed template review as done and a discarded one as skipped', async () => {
    const confirmed = await ownerHost();
    const applying = confirmed.bundles.applyPackage();
    await vi.waitFor(() => expect(confirmed.journal.draft.value).not.toBeNull());
    expect(confirmed.bundles.journal?.entries[0]?.status).toBe('running');
    await confirmed.journal.confirm(); await applying;
    expect(confirmed.bundles.journal?.entries[0]?.status).toBe('done');
    expect(confirmed.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);

    const discarded = await ownerHost();
    const second = discarded.bundles.applyPackage();
    await vi.waitFor(() => expect(discarded.journal.draft.value).not.toBeNull());
    discarded.journal.dismiss(); await second;
    expect(discarded.bundles.journal?.entries[0]).toMatchObject({ status: 'skipped', message: 'the template import was discarded' });
    expect(discarded.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
  });
});
