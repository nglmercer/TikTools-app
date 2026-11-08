import { describe, expect, test, vi } from 'vitest';
import type { BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { ControlCallError, type ResourceVersion } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { definitionJournal } from '@/entities/automation/model/definition-journal.ts';
import { libraryJournal } from './library-journal.ts';
import { libraryCommandHash, parseLibrarySnapshot, parseLibraryState, parseLibrarySubmission, type LibraryMutation } from './library-contracts.ts';
import { libraryPreview } from './library-preview.ts';
import { useRuleProfiles } from './rule-profiles-state.ts';
import { useRuleTemplates } from './rule-templates-state.ts';

const action = (id = 'old-action'): LiveAction => ({ schemaVersion: 2, id, name: id, enabled: false, typeId: 'core.log', config: { message: 'private-source' } });
const event = (id = 'old-event', actionIds = ['old-action']): LiveEvent => ({ schemaVersion: 1, id, name: id, enabled: false, trigger: 'tiktok.chat', filters: [], actionIds, cooldownMs: 0, cooldownScope: 'global', runMode: 'all' });
function fixture() {
  const source: BehaviorSnapshot = { actions: [action()], events: [event()], plugins: [], actionTypes: [], translations: {} };
  const revisions = new Map([['action:old-action', '1'], ['event:old-event', '1']]); const writes: string[] = [];
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: [...source.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: revisions.get(`action:${r.id}`)! })), ...source.events.map((r) => ({ kind: 'automation', scope: 'event', id: r.id, revision: revisions.get(`event:${r.id}`)! }))] });
      return structuredClone(source);
    }
    if (method === 'app.state.get') return { state: {} };
    writes.push(method);
    if (method === 'automation.set_enablement') {
      const changes = params?.updates as Array<{ kind: 'action' | 'event'; id: string; enabled: boolean }>;
      for (const change of changes) {
        const row = (change.kind === 'action' ? source.actions : source.events).find((row) => row.id === change.id)!;
        row.enabled = change.enabled; revisions.set(`${change.kind}:${change.id}`, String(BigInt(revisions.get(`${change.kind}:${change.id}`)!) + 1n));
      }
      return { updated: changes.length, missing: [] };
    }
    const kind = params?.kind as 'action' | 'event'; const record = params?.record as LiveAction | LiveEvent | undefined;
    const id = (record?.id ?? params?.id) as string; const key = `${kind}:${id}`;
    const list = (kind === 'action' ? source.actions : source.events) as Array<LiveAction | LiveEvent>; const index = list.findIndex((row) => row.id === id);
    if (method === 'automation.create' && index >= 0) throw new ControlCallError('revision_conflict', 'Already created');
    if (method === 'automation.delete') { if (index >= 0) list.splice(index, 1); }
    else if (record) { if (index < 0) list.push(structuredClone(record)); else list[index] = structuredClone(record); }
    const revision = String(BigInt(revisions.get(key) ?? '0') + 1n); revisions.set(key, revision);
    options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: kind, id, revision }] });
    return method === 'automation.delete' ? { id, deleted: index >= 0 } : structuredClone(record);
  });
  native.library.setCatalog({ packs: [{ id: 'a', name: 'Profile A', description: '', eventIds: ['old-event'], actionIds: ['old-action'] }, { id: 'b', name: 'Profile B', description: '', eventIds: [], actionIds: [] }], activeId: 'a', templates: [] });
  const fake = createFakeControlClient(native.handle); const journal = libraryJournal(fake.client);
  return { ...fake, native, source, journal, writes, revisions };
}
async function apply(f: ReturnType<typeof fixture>, mutation: LibraryMutation) {
  const context = await f.journal.refreshSnapshot(); const pending = f.journal.request(mutation, context); await f.journal.confirm(); await pending;
}

test('a known library commit keeps its review busy until a new coherent snapshot is available', async () => {
  const f = fixture(); let hold = false; let release!: () => void;
  const transport = createFakeControlClient(async (method, params, options) => {
    const response = await f.native.handle(method, params, options);
    if (hold && method === 'automation.library.snapshot') return new Promise<unknown>((resolve) => { release = () => resolve(response); });
    return response;
  });
  const journal = libraryJournal(transport.client); const context = await journal.refreshSnapshot();
  let finished = false;
  const pending = journal.request({ operation: 'create-profile', id: 'after-write', name: 'After write', description: '' }, context).then(() => { finished = true; });
  hold = true;
  const confirming = journal.confirm();
  await vi.waitFor(() => expect(release).toBeTypeOf('function'));
  expect(journal.busy.value).toBe(true); expect(journal.draft.value?.busy).toBe(true); expect(finished).toBe(false);
  expect(journal.snapshot.value?.catalog.packs.some((pack) => pack.id === 'after-write')).toBe(false);
  await expect(journal.request({ operation: 'create-profile', id: 'too-early', name: 'Too early', description: '' })).rejects.toMatchObject({ code: 'command_pending' });
  release(); await confirming; await pending;
  expect(finished).toBe(true); expect(journal.busy.value).toBe(false); expect(journal.draft.value).toBeNull();
  expect(journal.snapshot.value?.catalog.packs.some((pack) => pack.id === 'after-write')).toBe(true);
});

describe('atomic Owner library transition gateway', () => {
  test('all eight user operations use one native submission each and never write persistent app-state keys', async () => {
    const f = fixture();
    await apply(f, { operation: 'create-profile', id: 'c', name: 'Profile C', description: '' });
    await apply(f, { operation: 'switch-profile', id: 'b' });
    expect(f.source.actions[0]?.enabled).toBe(false);
    await apply(f, { operation: 'apply-template', profileId: 'b', entry: { actions: [action('act-frozen')], event: event('evt-frozen', ['act-frozen']) } });
    await apply(f, { operation: 'import-profile', profile: { id: 'a', name: 'Replacement A', description: '' }, entries: [{ actions: [action('act-import')], event: event('evt-import', ['act-import']) }] });
    expect(f.source.actions.some((row) => row.id === 'old-action')).toBe(true);
    expect(f.journal.snapshot.value?.catalog.packs.find((pack) => pack.id === 'a')?.actionIds).toEqual(['act-import']);
    await apply(f, { operation: 'assign-record', kind: 'action', id: 'old-action', profileId: 'c' });
    const template = { templateVersion: 1 as const, id: 'custom-template', title: { default: 'Saved template' }, description: { default: '' }, icon: 'plugin', actions: [{ name: 'Log', enabled: false, typeId: 'core.log', config: {} }], event: { name: 'Chat', enabled: false, trigger: 'tiktok.chat' as const, filters: [], cooldownMs: 0, cooldownScope: 'global' as const, runMode: 'all' as const } };
    await apply(f, { operation: 'import-templates', templates: [template] });
    await apply(f, { operation: 'delete-template', id: template.id });
    await apply(f, { operation: 'delete-profile', id: 'b' });
    expect(f.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(8);
    expect(f.calls.some((call) => call.method === 'app.state.set' || /^automation\.(create|delete|set_enablement)$/.test(call.method))).toBe(false);
    expect(f.journal.state.value?.recent).toHaveLength(8); expect(f.journal.snapshot.value?.revision).toBe('8');
  });

  test('lost reply retains the original graph IDs and candidate; reload/recovery never installs source or resends', async () => {
    const f = fixture(); const storing = vi.spyOn(Storage.prototype, 'setItem');
    const candidate: LibraryMutation = { operation: 'apply-template', profileId: 'b', entry: { actions: [action('act-original')], event: event('evt-original', ['act-original']) } };
    try {
      const context = await f.journal.refreshSnapshot();
      const pending = f.journal.request(candidate, context).catch((failure: unknown) => failure);
      candidate.entry.actions[0]!.name = 'Newer local input'; candidate.entry.event.id = 'evt-later';
      f.native.library.loseNextReply(); await f.journal.confirm();
      expect(f.journal.draft.value?.mutation).toMatchObject({ entry: { event: { id: 'evt-original' }, actions: [{ name: 'act-original' }] } });
      expect(f.journal.draft.value?.submitted).toBe(true); expect(f.journal.state.value?.recent[0]?.outcome).toBe('succeeded');
      await f.journal.confirm(); await f.journal.resolve(); await f.journal.readForReview();
      expect(f.journal.draft.value?.context).toBeDefined(); expect(f.journal.comparison.value?.definitions.events.some((row) => row.id === 'evt-original')).toBe(true);
      const reloaded = libraryJournal(createFakeControlClient(f.native.handle).client); await reloaded.refresh();
      expect(reloaded.snapshot.value).toBeNull(); expect(reloaded.state.value?.recent[0]?.intent.commandId).toBe(f.journal.state.value?.recent[0]?.intent.commandId);
      expect(f.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
      expect(storing).not.toHaveBeenCalled(); expect(JSON.stringify(reloaded.state.value)).not.toContain('private-source');
      f.journal.dismiss(); expect(await pending).toMatchObject({ code: 'unconfirmed' });
    } finally { storing.mockRestore(); }
  });

  test('a new definition invalidates the global switch fence without changing active profile or enablement', async () => {
    const f = fixture(); const context = await f.journal.refreshSnapshot();
    const pending = f.journal.request({ operation: 'switch-profile', id: 'b' }, context).catch((failure: unknown) => failure);
    f.source.actions.push(action('phantom')); f.revisions.set('action:phantom', '1');
    await f.journal.confirm();
    expect(f.journal.draft.value?.error).toContain('revision_conflict');
    expect(f.native.library.state().recent[0]?.outcome).toBe('rejected'); expect(f.writes).toEqual([]);
    expect(f.journal.draft.value?.context.definitionDigest).toBe(context.definitionDigest);
    f.journal.dismiss(); await pending;
  });

  test('a fresh late acknowledgement cannot discard a newer graph name typed while its original reply is held', async () => {
    const f = fixture(); let release!: () => void; let committed!: () => void;
    const hold = new Promise<void>((resolve) => { release = resolve; }); const ready = new Promise<void>((resolve) => { committed = resolve; });
    const transport = createFakeControlClient(async (method, params, options) => {
      const result = await f.native.handle(method, params, options);
      if (method === 'automation.library.submit') { committed(); await hold; }
      return result;
    });
    const journal = libraryJournal(transport.client); const context = await journal.refreshSnapshot();
    const pending = journal.request({ operation: 'apply-template', profileId: 'b', entry: { actions: [action('act-frozen')], event: event('evt-frozen', ['act-frozen']) } }, context);
    const writing = journal.confirm(); await ready;
    journal.renameRecord('event', 'evt-frozen', 'Newer name while saving'); release(); await writing; await pending;
    expect(f.source.events.find((row) => row.id === 'evt-frozen')?.name).toBe('evt-frozen');
    expect(journal.draft.value?.mutation).toMatchObject({ entry: { event: { id: 'evt-frozen', name: 'Newer name while saving' } } });
    expect(journal.draft.value).toMatchObject({ submitted: true, busy: false, reviewRequired: true });
    await journal.confirm(); expect(transport.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
    journal.dismiss();
  });

  test('unsafe source and missing reviewed context fail before requests; native failures never fall back', async () => {
    const f = fixture();
    await expect(f.journal.request({ operation: 'apply-template', profileId: 'a', entry: { actions: [{ ...action(), config: { integer: 9007199254740992 } }], event: event() } }, null)).rejects.toMatchObject({ code: 'capability_unavailable' });
    const context = await f.journal.refreshSnapshot(); f.calls.length = 0;
    await expect(f.journal.submit({ operation: 'apply-template', profileId: 'a', entry: { actions: [{ ...action(), config: { integer: 9007199254740992 } }], event: event() } }, context)).rejects.toMatchObject({ code: 'unsafe_source' });
    expect(f.calls).toHaveLength(0);
    const unknown = createFakeControlClient(() => { throw new ControlCallError('method_not_found', 'Unavailable'); });
    await expect(libraryJournal(unknown.client).submit({ operation: 'switch-profile', id: 'a' }, context)).rejects.toMatchObject({ code: 'method_not_found' });
    expect(unknown.calls.some((call) => call.method === 'automation.set_enablement' || call.method === 'app.state.set')).toBe(false);
  });

  test('definition writes cannot proceed through a pending library intent on the same original client', async () => {
    const f = fixture(); await apply(f, { operation: 'create-profile', id: 'c', name: 'C', description: '' });
    const stored = f.native.library.state(); f.native.library.setState({ ...stored, pending: { ...stored.recent[0]!.intent, commandId: `owner-library-${'f'.repeat(32)}` } });
    await expect(definitionJournal(f.client).submit({ operation: 'update', kind: 'action', id: 'old-action', expectedRevision: '1', record: action() }, { hostId: 'host-a', hostInstanceId: 'instance-a' })).rejects.toMatchObject({ code: 'command_pending' });
    expect(f.calls.some((call) => call.method === 'automation.definition.submit')).toBe(false);
  });
});

test('library metadata is closed, precise and bound to the fresh original receipt', async () => {
  const f = fixture(); const context = await f.journal.refreshSnapshot();
  expect(() => parseLibrarySnapshot({ ...context, source: {} })).toThrow();
  expect(() => parseLibrarySnapshot({ ...context, revision: 9007199254740992 })).toThrow();
  const mutation = { operation: 'create-profile' as const, id: 'c', name: 'C', description: '' };
  const raw = await f.native.library.handle('automation.library.submit', { mutation, expectedRevision: context.revision, definitionDigest: context.definitionDigest, hostId: context.hostId, hostInstanceId: context.hostInstanceId }) as { state: unknown; acknowledgement: { resourceVersions: ResourceVersion[] } };
  expect(parseLibrarySubmission(raw, mutation, context).acknowledgement).not.toBeNull();
  expect(() => parseLibraryState({ ...f.native.library.state(), source: 'private' })).toThrow();
  expect(() => parseLibrarySubmission(raw, { ...mutation, id: 'other' }, context)).toThrow();
  expect(() => parseLibrarySubmission(raw, { ...mutation, name: 'Changed captured body' }, context)).toThrow();
  expect(() => parseLibrarySnapshot({ ...context, definitions: { ...context.definitions, actions: [{ ...context.definitions.actions[0], config: { message: 'tampered snapshot' } }] } })).toThrow();
  expect(() => parseLibrarySubmission({ ...raw, acknowledgement: { ...raw.acknowledgement, resourceVersions: [...raw.acknowledgement.resourceVersions, raw.acknowledgement.resourceVersions[0]] } }, mutation, context)).toThrow();
});

test('profile deletion review includes external events whose action references change', async () => {
  const f = fixture(); f.source.events.push(event('external', ['old-action'])); f.revisions.set('event:external', '1');
  const preview = libraryPreview(await f.journal.refreshSnapshot(), { operation: 'delete-profile', id: 'a' });
  expect(preview.find((row) => row.name === 'external')).toMatchObject({ before: { actionIds: ['old-action'] }, after: { actionIds: [] } });
});

test('same-target membership acknowledgements bind kind and profile and reject altered final versions', async () => {
  const f = fixture(); const context = await f.journal.refreshSnapshot();
  const mutation = { operation: 'assign-record' as const, kind: 'action' as const, id: 'old-action', profileId: 'b' };
  const raw = await f.native.library.handle('automation.library.submit', { mutation, expectedRevision: context.revision, definitionDigest: context.definitionDigest, hostId: context.hostId, hostInstanceId: context.hostInstanceId }) as { state: unknown; acknowledgement: { resourceVersions: ResourceVersion[] } };
  expect(parseLibrarySubmission(raw, mutation, context).acknowledgement).not.toBeNull();
  expect(() => parseLibrarySubmission(raw, { ...mutation, kind: 'event' }, context)).toThrow();
  expect(() => parseLibrarySubmission(raw, { ...mutation, profileId: 'a' }, context)).toThrow();
  expect(() => parseLibrarySubmission({ ...raw, acknowledgement: { ...raw.acknowledgement, resourceVersions: raw.acknowledgement.resourceVersions.map((v) => ({ ...v, revision: '2' })) } }, mutation, context)).toThrow();
});

test('legacy profile/template helpers cannot write when persistence is unknown or enabled', async () => {
  const fake = createFakeControlClient(() => { throw new Error('No persistent legacy fallback'); });
  await expect(useRuleProfiles(fake.client).createPack('Profile')).rejects.toMatchObject({ code: 'capability_unavailable' });
  await expect(useRuleProfiles(fake.client, () => true).createPack('Profile')).rejects.toMatchObject({ code: 'capability_unavailable' });
  await useRuleTemplates(fake.client).deleteCustom('example');
  expect(fake.calls).toHaveLength(0);
});

test('legacy Unicode record IDs use scalar ordering in snapshot definitions and version associations', async () => {
  const f = fixture();
  f.source.actions = [action('\u{10000}'), action('\uE000')]; f.revisions.set('action:\u{10000}', '1'); f.revisions.set('action:\uE000', '1');
  const snapshot = await f.native.library.readSnapshot();
  expect(snapshot.resourceVersions.filter((version) => version.scope === 'action').map((version) => version.id)).toEqual(['\uE000', '\u{10000}']);
  expect(parseLibrarySnapshot(snapshot).definitions.actions.map((record) => record.id)).toEqual(['\u{10000}', '\uE000']);
  const wrongOrder = structuredClone(snapshot); [wrongOrder.resourceVersions[0], wrongOrder.resourceVersions[1]] = [wrongOrder.resourceVersions[1]!, wrongOrder.resourceVersions[0]!];
  expect(() => parseLibrarySnapshot(wrongOrder)).toThrow();
  const mutation: LibraryMutation = { operation: 'apply-template', profileId: 'a', entry: { actions: [action('act-new')], event: event('evt-new', ['act-new']) } };
  expect(libraryCommandHash(mutation, snapshot)).not.toBe(libraryCommandHash({ ...mutation, additionalEntries: [], templates: [] }, snapshot));
});
