import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { ControlCallError, type ControlClient } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { importLibraryDocument, readLibraryDocument, type LibraryImportEnvironment } from './library-import.ts';

const id = '00000000-0000-4000-8000-000000000001';
const sha256 = 'a'.repeat(64);
const template = { templateVersion: 1, id: 'cloud-template', title: 'Cloud template', description: '', icon: 'gift',
  actions: [{ name: 'act', typeId: 'core.log', config: { message: 'hi' } }], event: { name: 'evt', trigger: 'tiktok.chat' } };
const profile = { profileVersion: 1, id: 'cloud-profile', name: 'Cloud profile', description: 'two', params: {},
  templates: [{ template, params: {} }] };
const record = (document: unknown, revision = '3') => ({ meta: { id, kind: 'template', slug: 'x', revision, title: 'T', sha256, status: 'published' }, document });
function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (failure: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function fixture(read: (params: Record<string, unknown> | undefined) => unknown) {
  const source: BehaviorSnapshot = { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return structuredClone(source); }
    if (method === 'app.state.get') return { state: {} };
    if (method === 'automation.create') {
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: (params?.record as { id: string }).id, revision: '1' }] });
      return params?.record;
    }
    throw new Error(`unexpected ${method}`);
  });
  const fake = createFakeControlClient((method, params, options) => method === 'cloudLibrary.read' ? read(params) : native.handle(method, params, options));
  const state = { identity: { origin: 'https://cloud.example', accountId: 'alice' } as { origin: string; accountId: string } | null, stamp: 0 };
  const environment: LibraryImportEnvironment = {
    client: fake.client as ControlClient,
    identity: () => state.identity,
    stamp: () => state.stamp,
    isCurrent: (stamp) => stamp === state.stamp,
  };
  return { ...fake, native, state, environment, journal: libraryJournal(fake.client) };
}
const submits = (f: ReturnType<typeof fixture>) => f.calls.filter((call) => call.method === 'automation.library.submit');

describe('Cloud library import coordinator', () => {
  it('imports a template without visiting Automations, with real provenance, and settles only on confirmation', async () => {
    const f = fixture(() => record(template));
    let settled = false;
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3 }).then(() => { settled = true; });
    await flushPromises();
    expect(f.journal.draft.value?.mutation).toMatchObject({ operation: 'import-templates', templates: [{ id: 'cloud-template' }] });
    expect(f.journal.draft.value?.provenance).toEqual({ origin: 'https://cloud.example', accountId: 'alice', documentId: id, revision: '3', sha256 });
    expect(f.journal.draft.value?.context.hostId).toBe('host-a');
    expect(settled).toBe(false);
    expect(submits(f)).toHaveLength(0);
    await f.journal.confirm(); await outcome;
    expect(settled).toBe(true);
    expect(submits(f)).toHaveLength(1);
    expect(submits(f)[0]?.params?.provenance).toEqual({ origin: 'https://cloud.example', accountId: 'alice', documentId: id, revision: '3', sha256 });
  });

  it('imports a profile through the same review', async () => {
    const f = fixture(() => record(profile));
    const outcome = importLibraryDocument(f.environment, 'profile', { id, revision: '3' });
    await flushPromises();
    const mutation = f.journal.draft.value?.mutation;
    expect(mutation).toMatchObject({ operation: 'import-profile', profile: { id: 'cloud-profile' } });
    if (mutation?.operation !== 'import-profile') throw new Error('unexpected mutation');
    expect(mutation.entries).toHaveLength(1);
    await f.journal.confirm(); await outcome;
    expect(submits(f)).toHaveLength(1);
  });

  it('frees the flow when the review is discarded and sends nothing', async () => {
    const f = fixture(() => record(template));
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3 }).catch((failure: unknown) => failure);
    await flushPromises();
    f.journal.dismiss();
    expect(await outcome).toMatchObject({ code: 'cancelled' });
    expect(f.journal.draft.value).toBeNull();
    expect(submits(f)).toHaveLength(0);
    const again = importLibraryDocument(f.environment, 'template', { id, revision: 3 });
    await flushPromises();
    expect(f.journal.draft.value).not.toBeNull();
    await f.journal.confirm(); await again;
  });

  it('does not prepare a draft when the account changes while the document is being read', async () => {
    const read = deferred<unknown>();
    const f = fixture(() => read.promise);
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3 }).catch((failure: unknown) => failure);
    await flushPromises();
    f.state.identity = { origin: 'https://cloud.example', accountId: 'bob' }; f.state.stamp += 1;
    read.resolve(record(template));
    expect(await outcome).toMatchObject({ code: 'stale_context' });
    expect(f.journal.draft.value).toBeNull();
    expect(submits(f)).toHaveLength(0);
  });

  it('does not prepare a draft when a logout and login of the same account happened during the read', async () => {
    const read = deferred<unknown>();
    const f = fixture(() => read.promise);
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3 }).catch((failure: unknown) => failure);
    await flushPromises();
    f.state.stamp += 2;
    read.resolve(record(template));
    expect(await outcome).toMatchObject({ code: 'stale_context' });
    expect(f.journal.draft.value).toBeNull();
  });

  it('does not prepare a draft when the host changes while the document is being read', async () => {
    const read = deferred<unknown>();
    const f = fixture(() => read.promise);
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3 }).catch((failure: unknown) => failure);
    await flushPromises();
    f.native.replaceState({ ...f.native.state(), hostInstanceId: 'instance-b' });
    read.resolve(record(template));
    expect(await outcome).toMatchObject({ code: 'stale_context' });
    expect(f.journal.draft.value).toBeNull();
    expect(submits(f)).toHaveLength(0);
  });

  it('requires a signed-in account before reading anything', async () => {
    const f = fixture(() => record(template));
    f.state.identity = null;
    await expect(importLibraryDocument(f.environment, 'template', { id, revision: 3 })).rejects.toMatchObject({ code: 'signed_out', message: 'sign in to Cloud to import this template' });
    expect(f.calls).toHaveLength(0);
  });

  it('rejects a document that is not the revision or the digest requested', async () => {
    const wrongRevision = fixture(() => record(template, '4'));
    await expect(importLibraryDocument(wrongRevision.environment, 'template', { id, revision: 3 })).rejects.toMatchObject({ code: 'invalid_document' });
    const noDigest = fixture(() => ({ meta: { id, revision: '3', sha256: 'nope' }, document: template }));
    await expect(importLibraryDocument(noDigest.environment, 'template', { id, revision: 3 })).rejects.toMatchObject({ code: 'invalid_document' });
    expect(wrongRevision.journal.draft.value).toBeNull();
    expect(noDigest.journal.draft.value).toBeNull();
  });

  it('uses a document the caller already read without reading it again', async () => {
    const f = fixture(() => { throw new Error('must not read'); });
    const outcome = importLibraryDocument(f.environment, 'template', { id, revision: 3, record: record(template) });
    await flushPromises();
    expect(f.journal.draft.value?.provenance?.documentId).toBe(id);
    expect(f.calls.some((call) => call.method === 'cloudLibrary.read')).toBe(false);
    f.journal.dismiss(); await outcome.catch(() => {});
  });
});

describe('readLibraryDocument', () => {
  it('asks only the known scope', async () => {
    const f = fixture(() => record(template));
    await readLibraryDocument(f.environment.client, id, 3, 'mine');
    expect(f.calls.map((call) => call.params?.scope)).toEqual(['mine']);
  });

  it('tries the owner copy only when the public document is explicitly absent', async () => {
    const f = fixture((params) => { if (params?.scope === 'public') throw new ControlCallError('not_found', 'Cloud document not found'); return record(template); });
    await readLibraryDocument(f.environment.client, id, 3);
    expect(f.calls.map((call) => call.params?.scope)).toEqual(['public', 'mine']);
  });

  it('does not turn a network or permission failure into a private read', async () => {
    for (const code of ['unavailable', 'forbidden', 'timeout']) {
      const f = fixture(() => { throw new ControlCallError(code, 'Cloud library is unavailable'); });
      await expect(readLibraryDocument(f.environment.client, id, 3)).rejects.toMatchObject({ code });
      expect(f.calls.map((call) => call.params?.scope)).toEqual(['public']);
    }
  });
});
