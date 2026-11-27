import { expect, test } from 'vitest';

import type { ControlClient } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import {
  buildProfileDoc,
  DEFAULT_PROFILE_ID,
  instantiateProfile,
  mergedEntryParams,
  parseRuleProfile,
  parseRuleProfileImport,
  profileParamSchema,
  resolvedPacks,
  slugProfileId,
  useRuleProfiles,
} from '../../../../../../src/web/entities/rule-template/model/rule-profiles-state.ts';
import { ACTION_TYPE, ERROR_CODE, EVENT_KIND, RPC, defined } from '../../../../../shared/index.ts';
import { answer } from '../../../../support/answer.ts';
import { propertyOf } from '../../../../support/json.ts';

/** The key/value pair an `app.state.set` call carries. */
interface AppStatePair {
  key: string;
  value: string;
}

function appStatePair(params: unknown): AppStatePair {
  const key = propertyOf(params, 'key');
  const value = propertyOf(params, 'value');
  if (typeof key !== 'string' || typeof value !== 'string') throw new Error(`expected a key/value pair, got ${JSON.stringify(params)}`);
  return { key, value };
}

/** The per-record changes an `automation.set_enablement` call carries. */
interface EnablementUpdate {
  kind: string;
  id: string;
  enabled: boolean;
}

function enablementUpdates(params: unknown): EnablementUpdate[] {
  const updates = propertyOf(params, 'updates');
  if (!Array.isArray(updates)) throw new Error(`expected an updates list, got ${JSON.stringify(params)}`);
  return updates.map((update) => {
    const kind = propertyOf(update, 'kind');
    const id = propertyOf(update, 'id');
    const enabled = propertyOf(update, 'enabled');
    if (typeof kind !== 'string' || typeof id !== 'string' || typeof enabled !== 'boolean') throw new Error(`bad enablement update ${JSON.stringify(update)}`);
    return { kind, id, enabled };
  });
}

function fakeControl(
  handler: (method: string, params: unknown) => Promise<unknown>,
): {
  client: ControlClient;
  calls: Array<{
    method: string;
    params: unknown;
  }>;
} {
  const calls: Array<{
    method: string;
    params: unknown;
  }> = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: async <T>(method: string, params?: unknown): Promise<T> => {
      calls.push({ method, params });
      return answer<T>(await handler(method, params));
    },
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { client, calls };
}

const TEMPLATE = (id: string, gift: string) => ({
  templateVersion: 1,
  id,
  title: `${gift} template`,
  description: '',
  icon: 'gift',
  params: {
    type: 'object',
    properties: {
      giftName: { type: 'string', default: 'Rose' },
    },
  },
  actions: [{ name: 'act', typeId: ACTION_TYPE.coreLog, config: { message: gift } }],
  event: { name: 'evt', trigger: EVENT_KIND.tiktokGift, filters: [{ path: 'event.data.giftName', operator: 'eq', value: '{{ params.giftName }}' }] },
});

const PROFILE = {
  profileVersion: 1,
  id: 'board',
  name: 'Board',
  description: 'Two gifts',
  params: { giftName: 'Heart' },
  templates: [
    { template: TEMPLATE('tpl-one', 'one'), params: {} },
    { template: TEMPLATE('tpl-two', 'two'), params: { giftName: 'Galaxy' } },
  ],
};

test('profile import parses entries and rejects non-profiles', () => {
  const { profile, errors } = parseRuleProfileImport(JSON.stringify(PROFILE));
  expect(errors).toEqual([]);
  expect(profile?.templates.map((entry) => entry.template.id)).toEqual(['tpl-one', 'tpl-two']);

  expect(parseRuleProfileImport('nope').errors).toEqual(['import is not valid JSON']);
  expect(parseRuleProfileImport('x'.repeat(300000)).errors).toEqual(['import is too large (max 256 KB of text)']);
  expect(parseRuleProfileImport(JSON.stringify(TEMPLATE('solo', 'x'))).errors).toEqual([
    'this is a template document; import it as a template',
  ]);
  expect(parseRuleProfile({ ...PROFILE, profileVersion: 2 }).ok).toBe(false);
  expect(parseRuleProfile({ ...PROFILE, id: 'a' }).ok).toBe(true);
  expect(parseRuleProfile({ ...PROFILE, id: 'Perfil 🌹' }).ok).toBe(true);
  expect(parseRuleProfile({ ...PROFILE, id: 'default' }).ok).toBe(false);
  expect(parseRuleProfile({ ...PROFILE, templates: [] }).ok).toBe(false);
  expect(
    parseRuleProfile({ ...PROFILE, templates: [{ template: { nope: true }, params: {} }] }).ok,
  ).toBe(false);
});

test('param layers merge schema defaults, profile, then entry', () => {
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(mergedEntryParams(parsed.profile, defined(parsed.profile.templates[0]))).toEqual({ giftName: 'Heart' });
  expect(mergedEntryParams(parsed.profile, defined(parsed.profile.templates[1]))).toEqual({ giftName: 'Galaxy' });
});

test('import overrides win over entry params on every rule', () => {
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const [first, second] = instantiateProfile(parsed.profile, { giftName: 'TNT' });
  expect(first?.event.filters[0]?.value).toBe('TNT');
  expect(second?.event.filters[0]?.value).toBe('TNT');
});

test('profile param schema unions entries with profile values as defaults', () => {
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const collected = profileParamSchema(parsed.profile);
  expect(collected?.schema).toEqual({
    type: 'object',
    properties: { giftName: { type: 'string', default: 'Rose' } },
  });
  expect(collected?.defaults).toEqual({ giftName: 'Heart' });

  const plain = parseRuleProfile({ ...PROFILE, templates: [{ template: TEMPLATE('bare', 'x'), params: {} }] });
  expect(plain.ok).toBe(true);
  if (!plain.ok) return;
  defined(plain.profile.templates[0]).template.params = undefined;
  plain.profile.params = {};
  expect(profileParamSchema(plain.profile)).toBe(null);
});

test('instantiation substitutes params with fresh ids', () => {
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const [first, second] = instantiateProfile(parsed.profile);
  expect(first?.event.filters[0]?.value).toBe('Heart');
  expect(second?.event.filters[0]?.value).toBe('Galaxy');
  expect(first?.event.id).not.toBe(second?.event.id);
  expect(first?.event.actionIds).toEqual(first?.actions.map((action) => action.id));
});

test('register stores the pack and marks it active', async () => {
  const store: Record<string, string> = {};
  const { client } = fakeControl(async (method, params) => {
    if (method === RPC.appStateGet) return { state: store };
    if (method === RPC.appStateSet) {
      const { key, value } = appStatePair(params);
      store[key] = value;
      return { ok: true };
    }
    throw new Error(`unexpected ${method}`);
  });
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const state = useRuleProfiles(client, () => false);
  await state.registerPack(parsed.profile, instantiateProfile(parsed.profile));
  expect(state.activeId.value).toBe('board');
  expect(state.packs.value).toHaveLength(1);
  expect(state.packs.value[0]?.eventIds).toHaveLength(2);
  expect(state.packs.value[0]?.actionIds).toHaveLength(2);

  // Re-registering the same profile replaces membership (old ids unmanaged).
  await state.registerPack(parsed.profile, instantiateProfile(parsed.profile));
  expect(state.packs.value).toHaveLength(1);
});

test('resolved packs lead with the computed default leftovers', () => {
  const resolved = resolvedPacks(
    [{ id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: [] }],
    ['ea', 'loose'],
    ['aa'],
  );
  expect(resolved.map((pack) => pack.id)).toEqual([DEFAULT_PROFILE_ID, 'a']);
  expect(resolved[0]?.eventIds).toEqual(['loose']);
  expect(resolved[0]?.actionIds).toEqual(['aa']);
});

test('profile names slug to valid unique ids', () => {
  expect(slugProfileId('My Pack!')).toBe('my-pack');
  expect(slugProfileId('  ')).toBeNull();
  expect(slugProfileId('!!!')).toBeNull();
  expect(slugProfileId('default')).toBeNull();
  expect(slugProfileId('9lives')).toBe('9lives');
});

test('switch enables the target, disables everything else, prunes deleted rules', async () => {
  // Arrange: profile A owns ea and aa plus 'gone', a rule the host no longer has; profile B owns eb
  const store: Record<string, string> = {
    [RPC.behaviorProfilesPacks]: JSON.stringify([
      { id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: ['aa', 'gone'] },
      { id: 'b', name: 'B', description: '', eventIds: ['eb'], actionIds: [] },
    ]),
    [RPC.behaviorProfilesActive]: 'a',
  };
  const toggled: Array<{
    kind: string;
    id: string;
    enabled: boolean;
  }> = [];
  const { client } = fakeControl(async (method, params) => {
    if (method === RPC.appStateGet) return { state: store };
    if (method === RPC.appStateSet) {
      const { key, value } = appStatePair(params);
      store[key] = value;
      return { ok: true };
    }
    if (method === 'automation.set_enablement') {
      const updates = enablementUpdates(params);
      toggled.push(...updates);
      const missing = updates
        .filter((update) => update.id === 'gone')
        .map((update) => ({ kind: update.kind, id: update.id }));
      return { updated: updates.length - missing.length, missing };
    }
    throw new Error(`unexpected ${method}`);
  });
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  expect(state.activeId.value).toBe('a');
  // Act: switch to B, so A's rules are disabled, B's are enabled, and unowned 'loose' is disabled too
  await state.switchProfile('b', { eventIds: ['ea', 'eb', 'loose'], actionIds: ['aa'] });
  expect(toggled).toEqual([
    { kind: 'event', id: 'ea', enabled: false },
    { kind: 'event', id: 'eb', enabled: true },
    { kind: 'event', id: 'loose', enabled: false },
    { kind: 'action', id: 'aa', enabled: false },
    { kind: 'action', id: 'gone', enabled: false },
  ]);
  expect(state.activeId.value).toBe('b');
  // Assert: the deleted 'gone' rule is pruned from A without failing the switch or reporting an error
  expect(state.packs.value.find((pack) => pack.id === 'a')?.actionIds).toEqual(['aa']);
  expect(state.error.value).toBeNull();

  // Switching back to default revives exactly the leftovers.
  // Act: switch back to default, which re-enables only the leftovers that no named profile owns
  toggled.length = 0;
  await state.switchProfile(DEFAULT_PROFILE_ID, { eventIds: ['ea', 'eb', 'loose'], actionIds: ['aa'] });
  expect(toggled).toEqual([
    { kind: 'event', id: 'ea', enabled: false },
    { kind: 'event', id: 'eb', enabled: false },
    { kind: 'event', id: 'loose', enabled: true },
    { kind: 'action', id: 'aa', enabled: false },
  ]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
});

test('switch aborts without moving the active marker on host errors', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === RPC.appStateGet) {
      return {
        state: {
          [RPC.behaviorProfilesPacks]: JSON.stringify([
            { id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: [] },
          ]),
          [RPC.behaviorProfilesActive]: 'a',
        },
      };
    }
    throw new ControlCallError('internal', 'boom');
  });
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  await expect(state.switchProfile('a', { eventIds: ['ea'], actionIds: [] })).rejects.toThrow('boom');
  expect(state.activeId.value).toBe('a');
  expect(state.error.value).toBe('boom');
  await expect(state.switchProfile('missing', { eventIds: [], actionIds: [] })).rejects.toThrow('unknown profile');
});

test('delete removes the pack and its rules, falling back to default', async () => {
  const deleted: Array<{
    method: string;
    id: string;
  }> = [];
  const { client } = fakeControl(async (method, params) => {
    if (method === RPC.appStateGet) {
      return {
        state: {
          [RPC.behaviorProfilesPacks]: JSON.stringify([
            { id: 'a', name: 'A', description: '', eventIds: ['ea', 'gone'], actionIds: ['aa'] },
          ]),
          [RPC.behaviorProfilesActive]: 'a',
        },
      };
    }
    if (method === RPC.appStateSet) return { ok: true };
    if (method === RPC.automationDelete) {
      const id = propertyOf(params, 'id');
      if (typeof id !== 'string') throw new Error('expected a record id');
      if (id === 'gone') throw new ControlCallError(ERROR_CODE.automationNotFound, 'gone');
      deleted.push({ method, id });
      return { ok: true };
    }
    throw new Error(`unexpected ${method}`);
  });
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  await state.deletePack('a');
  expect(deleted).toEqual([
    { method: RPC.automationDelete, id: 'ea' },
    { method: RPC.automationDelete, id: 'aa' },
  ]);
  expect(state.packs.value).toEqual([]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
  await expect(state.deletePack(DEFAULT_PROFILE_ID)).rejects.toThrow('cannot be deleted');
  await expect(state.deletePack('missing')).rejects.toThrow('unknown profile');
});

test('create makes an empty pack with a unique slug and keeps active', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === RPC.appStateGet) {
      return {
        state: {
          [RPC.behaviorProfilesPacks]: JSON.stringify([{ id: 'board', name: 'Board', description: '', eventIds: [], actionIds: [] }]),
          [RPC.behaviorProfilesActive]: 'board',
        },
      };
    }
    return { ok: true };
  });
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  const pack = await state.createPack('Board');
  expect(pack.id).toBe('board-2');
  expect(pack.eventIds).toEqual([]);
  expect(state.activeId.value).toBe('board');
  await expect(state.createPack('   ')).rejects.toThrow('letter or digit');
});

test('hand-saved rules adopt into the active non-default pack only', async () => {
  const { client, calls } = fakeControl(async (method) => {
    if (method === RPC.appStateGet) {
      return {
        state: {
          [RPC.behaviorProfilesPacks]: JSON.stringify([{ id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: [] }]),
          [RPC.behaviorProfilesActive]: 'a',
        },
      };
    }
    return { ok: true };
  });
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  calls.length = 0;
  await state.adoptRule('event', 'fresh');
  expect(state.packs.value[0]?.eventIds).toEqual(['ea', 'fresh']);
  expect(calls.some((call) => call.method === RPC.appStateSet)).toBe(true);
  calls.length = 0;
  await state.adoptRule('event', 'ea');
  expect(calls).toEqual([]);
});

test('export mirrors the CLI profile shape and skips action-less events', () => {
  // Arrange: one event with a real action and a stray action ID, plus one event that has no actions
  const exported = buildProfileDoc(
    { id: 'pack-a', name: 'A', description: '', eventIds: ['e1', 'e2'], actionIds: ['a1'] },
    [
      {
        id: 'e1',
        name: 'First',
        enabled: true,
        trigger: EVENT_KIND.tiktokGift,
        filters: [],
        cooldownMs: 0,
        cooldownScope: 'global',
        runMode: 'all',
        actionIds: ['a1', 'stray'],
      },
      {
        id: 'e2',
        name: 'Lonely',
        enabled: false,
        trigger: EVENT_KIND.tiktokChat,
        filters: [],
        cooldownMs: 0,
        cooldownScope: 'global',
        runMode: 'all',
        actionIds: [],
      },
    ],
    [{ id: 'a1', name: 'Act', typeId: ACTION_TYPE.coreLog, enabled: true, config: { message: 'hi' } }],
  );
  // Assert: only the event with an action is exported; the action-less event is counted as skipped
  expect(exported.entries).toBe(1);
  expect(exported.skipped).toBe(1);
  expect(exported.doc['id']).toBe('pack-a');
  const templates = exported.doc['templates'];
  if (!Array.isArray(templates)) throw new Error('expected the exported document to list templates');
  const exportedTemplate = propertyOf(templates[0], 'template');
  // Assert: the template keeps the CLI's shape: a stable export ID, its one action, and no parameters
  expect(propertyOf(exportedTemplate, 'id')).toBe('export-e1');
  expect(propertyOf(exportedTemplate, 'actions')).toHaveLength(1);
  expect(propertyOf(templates[0], 'params')).toEqual({});
  // Round-trips through the importer.
  expect(parseRuleProfile(exported.doc).ok).toBe(true);
});

test('malformed sidecar loads empty with an error, active falls back to default', async () => {
  const { client } = fakeControl(async () => ({ state: { [RPC.behaviorProfilesPacks]: 'nope' } }));
  const state = useRuleProfiles(client, () => false);
  await state.loadProfiles();
  expect(state.packs.value).toEqual([]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
  expect(state.error.value).toContain('valid JSON');
});
