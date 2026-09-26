import { expect, test } from 'bun:test';

import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError } from '../platform/control-client.ts';
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
} from './rule-profiles.ts';

function fakeControl(
  handler: (method: string, params: unknown) => Promise<unknown>,
): { client: ControlClient; calls: Array<{ method: string; params: unknown }> } {
  const calls: Array<{ method: string; params: unknown }> = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: ((method: string, params?: unknown) => {
      calls.push({ method, params });
      return handler(method, params);
    }) as ControlClient['call'],
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
  actions: [{ name: 'act', typeId: 'core.log', config: { message: gift } }],
  event: { name: 'evt', trigger: 'tiktok.gift', filters: [{ path: 'event.data.giftName', operator: 'eq', value: '{{ params.giftName }}' }] },
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
  expect(parseRuleProfile({ ...PROFILE, templates: [] }).ok).toBe(false);
  expect(
    parseRuleProfile({ ...PROFILE, templates: [{ template: { nope: true }, params: {} }] }).ok,
  ).toBe(false);
});

test('param layers merge schema defaults, profile, then entry', () => {
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(mergedEntryParams(parsed.profile, parsed.profile.templates[0]!)).toEqual({ giftName: 'Heart' });
  expect(mergedEntryParams(parsed.profile, parsed.profile.templates[1]!)).toEqual({ giftName: 'Galaxy' });
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
  plain.profile.templates[0]!.template.params = undefined;
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
    if (method === 'app.state.get') return { state: store };
    if (method === 'app.state.set') {
      const { key, value } = params as { key: string; value: string };
      store[key] = value;
      return { ok: true };
    }
    throw new Error(`unexpected ${method}`);
  });
  const parsed = parseRuleProfile(PROFILE);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const state = useRuleProfiles(client);
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
  const store: Record<string, string> = {
    'behavior.profiles.packs': JSON.stringify([
      { id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: ['aa', 'gone'] },
      { id: 'b', name: 'B', description: '', eventIds: ['eb'], actionIds: [] },
    ]),
    'behavior.profiles.active': 'a',
  };
  const toggled: Array<{ method: string; id: string }> = [];
  const { client } = fakeControl(async (method, params) => {
    if (method === 'app.state.get') return { state: store };
    if (method === 'app.state.set') {
      const { key, value } = params as { key: string; value: string };
      store[key] = value;
      return { ok: true };
    }
    if (method === 'automation.enable' || method === 'automation.disable') {
      const { id } = params as { id: string };
      if (id === 'gone') throw new ControlCallError('automation_not_found', 'gone');
      toggled.push({ method, id });
      return { ok: true };
    }
    throw new Error(`unexpected ${method}`);
  });
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  expect(state.activeId.value).toBe('a');
  await state.switchProfile('b', { eventIds: ['ea', 'eb', 'loose'], actionIds: ['aa'] });
  expect(toggled).toEqual([
    { method: 'automation.disable', id: 'ea' },
    { method: 'automation.enable', id: 'eb' },
    { method: 'automation.disable', id: 'loose' },
    { method: 'automation.disable', id: 'aa' },
  ]);
  expect(state.activeId.value).toBe('b');
  expect(state.packs.value.find((pack) => pack.id === 'a')?.actionIds).toEqual(['aa']);
  expect(state.error.value).toBeNull();

  // Switching back to default revives exactly the leftovers.
  toggled.length = 0;
  await state.switchProfile(DEFAULT_PROFILE_ID, { eventIds: ['ea', 'eb', 'loose'], actionIds: ['aa'] });
  expect(toggled).toEqual([
    { method: 'automation.disable', id: 'ea' },
    { method: 'automation.disable', id: 'eb' },
    { method: 'automation.enable', id: 'loose' },
    { method: 'automation.disable', id: 'aa' },
  ]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
});

test('switch aborts without moving the active marker on host errors', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === 'app.state.get') {
      return {
        state: {
          'behavior.profiles.packs': JSON.stringify([
            { id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: [] },
          ]),
          'behavior.profiles.active': 'a',
        },
      };
    }
    throw new ControlCallError('internal', 'boom');
  });
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  await expect(state.switchProfile('a', { eventIds: ['ea'], actionIds: [] })).rejects.toThrow('boom');
  expect(state.activeId.value).toBe('a');
  expect(state.error.value).toBe('boom');
  await expect(state.switchProfile('missing', { eventIds: [], actionIds: [] })).rejects.toThrow('unknown profile');
});

test('delete removes the pack and its rules, falling back to default', async () => {
  const deleted: Array<{ method: string; id: string }> = [];
  const { client } = fakeControl(async (method, params) => {
    if (method === 'app.state.get') {
      return {
        state: {
          'behavior.profiles.packs': JSON.stringify([
            { id: 'a', name: 'A', description: '', eventIds: ['ea', 'gone'], actionIds: ['aa'] },
          ]),
          'behavior.profiles.active': 'a',
        },
      };
    }
    if (method === 'app.state.set') return { ok: true };
    if (method === 'automation.delete') {
      const { id } = params as { id: string };
      if (id === 'gone') throw new ControlCallError('automation_not_found', 'gone');
      deleted.push({ method, id });
      return { ok: true };
    }
    throw new Error(`unexpected ${method}`);
  });
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  await state.deletePack('a');
  expect(deleted).toEqual([
    { method: 'automation.delete', id: 'ea' },
    { method: 'automation.delete', id: 'aa' },
  ]);
  expect(state.packs.value).toEqual([]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
  await expect(state.deletePack(DEFAULT_PROFILE_ID)).rejects.toThrow('cannot be deleted');
  await expect(state.deletePack('missing')).rejects.toThrow('unknown profile');
});

test('create makes an empty pack with a unique slug and keeps active', async () => {
  const { client } = fakeControl(async (method) => {
    if (method === 'app.state.get') {
      return {
        state: {
          'behavior.profiles.packs': JSON.stringify([{ id: 'board', name: 'Board', description: '', eventIds: [], actionIds: [] }]),
          'behavior.profiles.active': 'board',
        },
      };
    }
    return { ok: true };
  });
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  const pack = await state.createPack('Board');
  expect(pack.id).toBe('board-2');
  expect(pack.eventIds).toEqual([]);
  expect(state.activeId.value).toBe('board');
  await expect(state.createPack('   ')).rejects.toThrow('letter or digit');
});

test('hand-saved rules adopt into the active non-default pack only', async () => {
  const { client, calls } = fakeControl(async (method) => {
    if (method === 'app.state.get') {
      return {
        state: {
          'behavior.profiles.packs': JSON.stringify([{ id: 'a', name: 'A', description: '', eventIds: ['ea'], actionIds: [] }]),
          'behavior.profiles.active': 'a',
        },
      };
    }
    return { ok: true };
  });
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  calls.length = 0;
  await state.adoptRule('event', 'fresh');
  expect(state.packs.value[0]?.eventIds).toEqual(['ea', 'fresh']);
  expect(calls.some((call) => call.method === 'app.state.set')).toBe(true);
  calls.length = 0;
  await state.adoptRule('event', 'ea');
  expect(calls).toEqual([]);
});

test('export mirrors the CLI profile shape and skips action-less events', () => {
  const exported = buildProfileDoc(
    { id: 'pack-a', name: 'A', description: '', eventIds: ['e1', 'e2'], actionIds: ['a1'] },
    [
      { id: 'e1', name: 'First', enabled: true, trigger: 'tiktok.gift', filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all', actionIds: ['a1', 'stray'] },
      { id: 'e2', name: 'Lonely', enabled: false, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all', actionIds: [] },
    ],
    [{ id: 'a1', name: 'Act', typeId: 'core.log', enabled: true, config: { message: 'hi' } }],
  );
  expect(exported.entries).toBe(1);
  expect(exported.skipped).toBe(1);
  expect(exported.doc['id']).toBe('pack-a');
  const templates = exported.doc['templates'] as Array<{ template: { id: string; actions: unknown[] }; params: unknown }>;
  expect(templates[0]?.template.id).toBe('export-e1');
  expect(templates[0]?.template.actions).toHaveLength(1);
  expect(templates[0]?.params).toEqual({});
  // Round-trips through the importer.
  expect(parseRuleProfile(exported.doc).ok).toBe(true);
});

test('malformed sidecar loads empty with an error, active falls back to default', async () => {
  const { client } = fakeControl(async () => ({ state: { 'behavior.profiles.packs': 'nope' } }));
  const state = useRuleProfiles(client);
  await state.loadProfiles();
  expect(state.packs.value).toEqual([]);
  expect(state.activeId.value).toBe(DEFAULT_PROFILE_ID);
  expect(state.error.value).toContain('valid JSON');
});
