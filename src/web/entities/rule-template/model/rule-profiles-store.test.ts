import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { expect, test } from 'vitest';
import type { BehaviorSnapshot, LiveAction } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useRuleProfilesStore } from './rule-profiles-store.ts';
import { useAutomationEditor } from '@/features/edit-automation-record/model/automation-editor.ts';
import { libraryJournal } from './library-journal.ts';
import { flushPromises } from '@vue/test-utils';

test('a delayed saved action adopts into the originating profile, independently of same-ID event membership', async () => {
  const snapshot: BehaviorSnapshot = { plugins: [], actions: [], actionTypes: [], translations: {}, events: [{
    schemaVersion: 1, id: 'same', name: 'Event in A', enabled: false, trigger: 'tiktok.chat', filters: [],
    cooldownMs: 0, cooldownScope: 'user', runMode: 'all', actionIds: [],
  }] };
  const state: Record<string, string> = {
    'behavior.profiles.packs': JSON.stringify([
      { id: 'a', name: 'A', description: '', eventIds: ['same'], actionIds: [] },
      { id: 'b', name: 'B', description: '', eventIds: [], actionIds: [] },
    ]),
    'behavior.profiles.active': 'a',
  };
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => { finish = resolve; });
  const { client } = createFakeControlClient(nativeDefinitionHost((method, params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'app.state.get') return { state: { ...state } };
    if (method === 'app.state.set') { state[params!['key'] as string] = params!['value'] as string; return { ok: true }; }
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'automation.set_enablement') return { updated: 1, missing: [] };
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: [
        { kind: 'automation', scope: 'event', id: 'same', revision: '1' },
        ...snapshot.actions.map((r) => ({ kind: 'automation', scope: 'action', id: r.id, revision: '1' })),
      ] });
      return structuredClone(snapshot);
    }
    if (method === 'automation.create') {
      const record = params!['record'] as LiveAction; snapshot.actions = [record];
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'action', id: 'same', revision: '1' }] });
      return pending.then(() => record);
    }
    throw new Error(`unexpected ${method}`);
  }).handle);
  const harness = createStoreHarness(client);
  const automation = harness.use(() => useAutomationStore());
  const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  const action: LiveAction = { schemaVersion: 2, id: 'same', name: 'New action in A', enabled: false, typeId: 'core.log', config: {} };
  const saving = profiles.saveAction(action, { create: true, revision: '0', host: automation.sourceHost });
  await expect(profiles.switchProfile('b')).rejects.toMatchObject({ code: 'command_pending' });
  finish(); await saving;
  const library = libraryJournal(client);
  expect(library.draft.value?.mutation).toMatchObject({ operation: 'assign-record', kind: 'action', id: 'same', profileId: 'a' });
  await library.confirm();
  expect(profiles.activeId).toBe('a');
  expect(profiles.packs.find((p) => p.id === 'a')?.actionIds).toEqual(['same']);
  expect(profiles.packs.find((p) => p.id === 'b')?.actionIds).toEqual([]);
  automation.dispose();
});

test('a profile assignment failure reports the saved definition separately and never makes its editor repeat the save', async () => {
  let stored: LiveAction | null = null; let writes = 0;
  const profileState = { 'behavior.profiles.packs': JSON.stringify([{ id: 'a', name: 'A', description: '', eventIds: [], actionIds: [] }]), 'behavior.profiles.active': 'a' };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'app.state.get') return { state: profileState };
    if (method === 'app.state.set') throw new Error('profile assignment offline');
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: stored ? [{ kind: 'automation', scope: 'action', id: stored.id, revision: '1' }] : [] });
      return { actions: stored ? [stored] : [], events: [], plugins: [], actionTypes: [], translations: {} };
    }
    if (method === 'automation.create') {
      ++writes; stored = params?.record as LiveAction;
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'action', id: stored.id, revision: '1' }] });
      return stored;
    }
    throw new Error(`unexpected ${method}`);
  });
  const { client, calls } = createFakeControlClient((method, params, options) => {
    if (method === 'automation.library.submit') throw new Error('profile assignment offline');
    return native.handle(method, params, options);
  });
  const harness = createStoreHarness(client);
  const automation = harness.use(() => useAutomationStore()); const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  const editor = useAutomationEditor({ read: automation.readRecord,
    save: (_kind, record, base) => profiles.saveAction(record as LiveAction, base), remove: automation.deleteRecord });
  editor.open('action', { schemaVersion: 2, id: 'saved', name: 'Saved definition', typeId: 'core.log', enabled: false, config: {} }, '0', true, automation.sourceHost);
  await editor.save();
  expect(writes).toBe(1); expect(editor.state.value?.revision).toBe('1'); expect(editor.state.value?.reviewRequired).toBe(false);
  expect(editor.state.value?.create).toBe(false);
  const library = libraryJournal(client); expect(library.draft.value?.mutation.operation).toBe('assign-record');
  await library.confirm(); library.dismiss(); await flushPromises();
  expect(profiles.error).toContain('The definition was saved.');
  expect(profiles.error).toContain('profile assignment offline');
  await editor.save();
  expect(writes).toBe(1); expect(calls.filter((c) => c.method === 'automation.definition.submit')).toHaveLength(1);
  editor.dispose(); automation.dispose();
});

test.each([false, true])('membership review binds a fresh saved record after a superseded save refresh and preserves a newer review (%s)', async (newerReview) => {
  let stored: LiveAction | null = null;
  const raw = { 'behavior.profiles.packs': JSON.stringify([{ id: 'a', name: 'A', description: '', eventIds: [], actionIds: [] }]), 'behavior.profiles.active': 'a' };
  const native = nativeDefinitionHost((method, params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'app.state.get') return { state: raw };
    if (method === 'automation.snapshot') {
      options?.onMetadata?.({ resourceVersions: stored ? [{ kind: 'automation', scope: 'action', id: stored.id, revision: '1' }] : [] });
      return { actions: stored ? [stored] : [], events: [], plugins: [], actionTypes: [], translations: {} };
    }
    if (method === 'automation.create') {
      stored = structuredClone(params?.record as LiveAction);
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'action', id: stored.id, revision: '1' }] });
      return stored;
    }
    throw new Error(`Unexpected ${method}`);
  });
  const held: Array<() => void> = [];
  const fake = createFakeControlClient(async (method, params, options) => {
    const response = await native.handle(method, params, options);
    if (method === 'automation.library.snapshot' && stored && held.length < 2) {
      return new Promise<unknown>((resolve) => { held.push(() => resolve(response)); });
    }
    return response;
  });
  const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore()); const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  const action: LiveAction = { schemaVersion: 2, id: 'fresh-action', name: 'Known saved action', enabled: false, typeId: 'core.log', config: {} };
  const saving = profiles.saveAction(action, { create: true, revision: '0', host: automation.sourceHost });
  await flushPromises(); expect(held).toHaveLength(1);
  action.name = 'Newer unsaved name';
  const library = libraryJournal(fake.client);
  const otherReview = newerReview ? library.request({ operation: 'create-profile', id: 'newer-review', name: 'Newer review', description: '' }) : null;
  const background = automation.refresh(); await flushPromises(); expect(held).toHaveLength(2);
  held[0]!(); const result = await saving;
  expect(result.revision).toBe('1');
  expect(result.record?.name).toBe('Known saved action'); expect(action.name).toBe('Newer unsaved name');
  if (newerReview) {
    expect(library.draft.value?.mutation).toMatchObject({ operation: 'create-profile', id: 'newer-review', name: 'Newer review' });
    expect(profiles.error).toContain('The definition was saved.');
  } else {
    expect(library.draft.value?.context.definitions.actions).toEqual([{ ...action, name: 'Known saved action' }]);
    expect(library.draft.value?.context.resourceVersions).toContainEqual({ kind: 'automation', scope: 'action', id: action.id, revision: '1' });
    expect(library.draft.value?.mutation).toMatchObject({ operation: 'assign-record', id: action.id, profileId: 'a' });
  }
  held[1]!(); await background;
  if (otherReview) {
    library.dismiss(); await expect(otherReview).rejects.toMatchObject({ code: 'cancelled' });
    expect(fake.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
  } else {
    await library.confirm(); await flushPromises();
    expect(profiles.packs[0]?.actionIds).toEqual([action.id]);
    expect(fake.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
  }
  expect(fake.calls.filter((call) => call.method === 'automation.definition.submit')).toHaveLength(1);
  automation.dispose();
});

test('a post-save snapshot missing the acknowledged target reports only membership failure and retains the save acknowledgement', async () => {
  const raw = { 'behavior.profiles.packs': JSON.stringify([{ id: 'a', name: 'A', description: '', eventIds: [], actionIds: [] }]), 'behavior.profiles.active': 'a' };
  const action: LiveAction = { schemaVersion: 2, id: 'known-saved', name: 'Known saved', enabled: false, typeId: 'core.log', config: {} };
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === 'system.info') return { features: { persistence: true } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'app.state.get') return { state: raw };
    if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
    if (method === 'automation.create') { options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'action', id: action.id, revision: '1' }] }); return action; }
    throw new Error(`Unexpected ${method}`);
  });
  const fake = createFakeControlClient(native.handle); const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore()); const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  const result = await profiles.saveAction(action, { create: true, revision: '0', host: automation.sourceHost });
  expect(result.record).toEqual(action); expect(result.revision).toBe('1');
  expect(profiles.error).toContain('The definition was saved.');
  expect(libraryJournal(fake.client).draft.value).toBeNull();
  expect(fake.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
  automation.dispose();
});

test.each([false, true])('64-character profile name collisions terminate with distinct bounded IDs (persistence=%s)', async (persistence) => {
  const longest = 'a'.repeat(64);
  const raw = { 'behavior.profiles.packs': JSON.stringify([{ id: longest, name: longest, description: '', eventIds: [], actionIds: [] }]), 'behavior.profiles.active': longest };
  const native = nativeDefinitionHost((method, _params, options) => {
    if (method === 'system.info') return { features: { persistence } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'app.state.get') return { state: raw };
    if (method === 'app.state.set') { if (persistence) throw new Error('No persistent legacy write'); return { ok: true }; }
    if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
    throw new Error(`Unexpected method ${method}`);
  });
  const fake = createFakeControlClient(native.handle); const harness = createStoreHarness(fake.client);
  const automation = harness.use(() => useAutomationStore()); const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  expect(profiles.loading).toBe(false);
  if (!persistence) {
    expect(libraryJournal(fake.client).snapshot.value).toBeNull();
    expect(fake.calls.some((call) => call.method === 'automation.library.snapshot')).toBe(false);
  }
  for (let i = 0; i < 2; ++i) {
    const creating = profiles.createProfile(longest);
    if (persistence) await libraryJournal(fake.client).confirm();
    await creating;
  }
  const ids = profiles.packs.map((pack) => pack.id);
  expect(ids).toHaveLength(3); expect(new Set(ids).size).toBe(3); expect(ids.every((id) => id.length <= 64)).toBe(true);
  expect(ids).toContain(longest); expect(ids).toContain(`${'a'.repeat(62)}-2`); expect(ids).toContain(`${'a'.repeat(62)}-3`);
  expect(fake.calls.some((call) => call.method === 'app.state.set')).toBe(!persistence);
  automation.dispose();
});
