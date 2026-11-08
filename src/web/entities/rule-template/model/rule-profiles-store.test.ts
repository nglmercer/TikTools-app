import { expect, test } from 'vitest';
import type { BehaviorSnapshot, LiveAction } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useRuleProfilesStore } from './rule-profiles-store.ts';

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
  const { client } = createFakeControlClient((method, params, options) => {
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
  });
  const harness = createStoreHarness(client);
  const automation = harness.use(() => useAutomationStore());
  const profiles = harness.use(() => useRuleProfilesStore());
  await automation.refresh(); await profiles.loadProfiles();
  const action: LiveAction = { schemaVersion: 2, id: 'same', name: 'New action in A', enabled: false, typeId: 'core.log', config: {} };
  const saving = profiles.saveAction(action, { create: true, revision: '0' });
  await profiles.switchProfile('b');
  expect(profiles.activeId).toBe('b');
  finish(); await saving;
  expect(profiles.activeId).toBe('b');
  expect(profiles.packs.find((p) => p.id === 'a')?.actionIds).toEqual(['same']);
  expect(profiles.packs.find((p) => p.id === 'b')?.actionIds).toEqual([]);
  automation.dispose();
});
