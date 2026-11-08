import { expect, test } from 'vitest';
import { watch } from 'vue';
import type { BehaviorSnapshot, LiveAction } from '../../../../automation/behavior/types.ts';
import { ControlCallError, type ControlCallOptions, type RpcParams } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { useAutomation } from './automation-state.ts';

const original: LiveAction = { id: 'same', name: 'Initial', schemaVersion: 2, enabled: true, typeId: 'core.log', config: {} };
function fixture() {
  const snapshot: BehaviorSnapshot = { actions: [original], events: [], actionTypes: [], plugins: [], translations: {} };
  let revision = '1';
  let persistence: boolean | undefined = true;
  let metadata = true;
  let missing = false;
  let holdNextSnapshot: Promise<void> | undefined;
  const requests: Array<{ method: string; meta: ControlCallOptions['meta']; params?: RpcParams }> = [];
  const meta = (id = 'same') => ({ resourceVersions: [{ kind: 'automation', scope: 'action', id, revision }] });
  const { client } = createFakeControlClient((method, params, options) => {
    requests.push({ method, params, meta: options?.meta });
    if (method === 'system.info') return { features: { persistence } };
    if (method === 'automation.runs') return { runs: [] };
    if (method === 'automation.snapshot') {
      const body = structuredClone(snapshot);
      if (metadata) options?.onMetadata?.(meta());
      const hold = holdNextSnapshot; holdNextSnapshot = undefined;
      return hold ? hold.then(() => body) : body;
    }
    if (method === 'automation.get') {
      if (missing) throw new ControlCallError('automation_not_found', 'Deleted', meta());
      if (metadata) options?.onMetadata?.(meta());
      return structuredClone(snapshot.actions[0]);
    }
    if (method === 'automation.update' || method === 'automation.create') {
      if (options?.meta?.expectedRevision !== (method === 'automation.create' ? (missing ? revision : '0') : revision) && persistence !== false) {
        throw new ControlCallError('revision_conflict', 'Host changed', meta());
      }
      if (method === 'automation.update' && missing) throw new ControlCallError('automation_not_found', 'Deleted', meta());
      revision = String(BigInt(method === 'automation.create' && !missing ? '0' : revision) + 1n);
      const record = structuredClone(params?.record) as LiveAction;
      snapshot.actions = [record]; missing = false;
      if (metadata) options?.onMetadata?.(meta(record.id));
      return record;
    }
    throw new Error(`unexpected ${method}`);
  });
  const automation = useAutomation(client);
  return { automation, snapshot, requests,
    changeHost(name: string, next: string) { snapshot.actions = [{ ...original, name }]; revision = next; },
    setMissing(next: string) { missing = true; revision = next; snapshot.actions = []; },
    setPersistence(value: boolean | undefined) { persistence = value; },
    setMetadata(value: boolean) { metadata = value; },
    holdSnapshot(value: Promise<void>) { holdNextSnapshot = value; },
  };
}

test('body and revisions become visible in one projection, including synchronous observers', async () => {
  const f = fixture(); await f.automation.refresh();
  const seen: Array<[string | undefined, unknown]> = [];
  const stop = watch(() => f.automation.behavior.value, (body) => {
    seen.push([body.actions[0]?.name, f.automation.revisionFor('action', 'same')]);
  }, { flush: 'sync' });
  f.changeHost('Second', '2'); await f.automation.refresh();
  expect(seen).toEqual([['Second', '2']]);
  expect(f.automation.revisionFor('event', 'same')).toBeUndefined();
  stop(); f.automation.dispose();
});

test('saved-base updates keep their precondition after refresh and never recreate a missing update', async () => {
  const f = fixture(); await f.automation.refresh();
  const base = f.automation.revisionFor('action', 'same'); expect(base).toBe('1');
  f.changeHost('Host change', '2'); await f.automation.refresh();
  await expect(f.automation.saveRecordAndRefresh('action', { ...original, name: 'Draft' }, { revision: base!, create: false })).rejects.toMatchObject({ code: 'revision_conflict' });
  f.setMissing('2');
  await expect(f.automation.saveRecordAndRefresh('action', original, { revision: '2', create: false })).rejects.toMatchObject({ code: 'automation_not_found' });
  expect(f.requests.filter((r) => r.method === 'automation.update').map((r) => r.meta)).toEqual([{ expectedRevision: '1' }, { expectedRevision: '2' }]);
  expect(f.requests.some((r) => r.method === 'automation.create')).toBe(false);
  f.automation.dispose();
});

test('new records use explicit create with zero; missing records expose the retained tombstone for reviewed recreation', async () => {
  const f = fixture(); await f.automation.refresh();
  await f.automation.saveRecordAndRefresh('action', { ...original, id: 'fresh' }, { revision: '0', create: true });
  expect(f.requests.find((r) => r.method === 'automation.create')?.meta).toEqual({ expectedRevision: '0' });
  f.setMissing('4');
  expect(await f.automation.readRecord('action', 'same')).toEqual({ record: null, revision: '4' });
  const result = await f.automation.saveRecordAndRefresh('action', original, { revision: '4', create: true });
  expect(result.revision).toBe('5');
  f.automation.dispose();
});

test('a successful body without revision acknowledgement is unconfirmed and cannot silently downgrade CAS', async () => {
  const f = fixture(); await f.automation.refresh();
  f.setMetadata(false);
  await expect(f.automation.saveRecordAndRefresh('action', original, { revision: '1', create: false })).rejects.toMatchObject({ code: 'protocol' });
  expect(f.requests.filter((r) => r.method === 'automation.update')).toHaveLength(1);
  expect(f.requests.find((r) => r.method === 'automation.update')?.meta).toEqual({ expectedRevision: '1' });
  await f.automation.refresh();
  expect(f.automation.revisionFor('action', 'same')).toBeUndefined();
  f.automation.dispose();
});

test('only explicitly non-persistent owner hosts allow an unversioned memory edit', async () => {
  for (const persistence of [false, true, undefined]) {
    const f = fixture(); f.setMetadata(false); f.setPersistence(persistence); await f.automation.refresh();
    const saving = f.automation.saveRecordAndRefresh('action', original, { revision: null, create: false });
    if (persistence === false) {
      expect(f.automation.revisionFor('action', 'same')).toBe(null);
      expect((await saving).revision).toBe(null);
      expect(f.requests.find((r) => r.method === 'automation.update')?.meta).toBeUndefined();
    } else {
      await expect(saving).rejects.toMatchObject({ code: 'capability_unavailable' });
      expect(f.requests.some((r) => r.method === 'automation.update')).toBe(false);
    }
    f.automation.dispose();
  }
});

test('a snapshot started before a write cannot rewind the acknowledged projection when it completes later', async () => {
  const f = fixture(); await f.automation.refresh();
  let release!: () => void;
  f.holdSnapshot(new Promise<void>((resolve) => { release = resolve; }));
  const oldRead = f.automation.refresh();
  await f.automation.saveRecordAndRefresh('action', { ...original, name: 'Committed new body' }, { revision: '1', create: false });
  release(); await oldRead;
  expect(f.automation.behavior.value.actions[0]?.name).toBe('Committed new body');
  expect(f.automation.revisionFor('action', 'same')).toBe('2');
  f.automation.dispose();
});
