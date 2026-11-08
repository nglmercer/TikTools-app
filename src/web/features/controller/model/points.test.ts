import { afterEach, expect, it } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createControllerModel, type ControllerStatus, type PointsPage } from './controller.ts';

const models: ReturnType<typeof createControllerModel>[] = [];
afterEach(() => { models.splice(0).forEach((m) => m.stop()); });
function fixture() {
  const status: ControllerStatus = {
    epoch: '1', viewRevision: '1', pointsRevision: '1', pointsLoading: false, phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection-1', grantRevision: '1', hostId: 'host', principalId: 'mod', sessionId: 'pc-B', hostInstanceId: 'instance',
      capabilities: ['points.read', 'points.adjust', 'commands.read'], features: { pointsProjection: true, pointsAdjust: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host', principalId: 'mod', sessionId: 'pc-B' }, pending: null, recent: [], unavailable: false },
  };
  const version = { kind: 'viewer', scope: 'host', id: 'alice', revision: '1' };
  const points: PointsPage = { epoch: '1', pointsRevision: '1', loading: false, errorCode: null, currencyName: 'Gems', rows: [], selectedId: 'alice',
    selected: { viewer: { uniqueId: 'alice', points: 10, level: 1 }, version }, selectedVersion: version };
  let lost = false;
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.points') return structuredClone(points);
    if (method === 'controller.points.select') {
      points.selectedId = params?.['uniqueId'] as string | null;
      points.selected = null;
      points.selectedVersion = points.selectedId ? { ...version, id: points.selectedId, revision: '0' } : null;
      points.pointsRevision = status.pointsRevision = String(Number(status.pointsRevision) + 1); return {};
    }
    if (method === 'controller.points.command') {
      if (lost) {
        status.commandJournal!.pending = { commandId: 'native-id', operation: 'adjust-points', resourceId: String(params?.['uniqueId']), delta: Number(params?.['delta']), currencyName: 'Gems', hostInstanceId: 'instance', createdAt: 1 };
        throw new ControlCallError('transport', 'Acknowledgement lost');
      }
      return {};
    }
    throw new Error(`Unexpected call: ${method}`);
  });
  const model = createControllerModel(client); models.push(model);
  return { model, status, points, calls, lose: () => { lost = true; } };
}

it('keeps an additive confirmation valid when only the balance changes and sends no JS command ID or CAS', async () => {
  const f = fixture(); await f.model.start();
  const t = f.model.preparePoints('alice', -3)!; expect(t).toMatchObject({ uniqueId: 'alice', delta: -3, currencyName: 'Gems', creates: false });
  f.points.selected!.viewer.points = 500; f.points.pointsRevision = f.status.pointsRevision = '2';
  await f.model.refresh();
  expect(f.model.pointsIsCurrent(t)).toBe(true);
  expect(await f.model.adjustPoints(t)).toBe(true);
  expect(f.calls.filter((c) => c.method === 'controller.points.command').map((c) => c.params)).toEqual([
    { epoch: '1', connectionId: 'connection-1', uniqueId: 'alice', delta: -3, currencyName: 'Gems' },
  ]);
  expect(f.calls.some((c) => c.method === 'controller.forward')).toBe(false);
});

it.each(['connectionId', 'grantRevision', 'hostId', 'hostInstanceId', 'principalId', 'sessionId'] as const)('invalidates a reviewed points target after %s changes', async (field) => {
  const f = fixture(); await f.model.start(); const t = f.model.preparePoints('alice', 2)!;
  f.status.hello![field] = 'changed'; await f.model.refresh();
  expect(f.model.pointsIsCurrent(t)).toBe(false); expect(await f.model.adjustPoints(t)).toBe(false);
  expect(f.calls.some((c) => c.method === 'controller.points.command')).toBe(false);
});

it('invalidates unit/scope changes, marks a new viewer, normalizes lookup aliases and blocks unresolved writes', async () => {
  const f = fixture(); await f.model.start(); const old = f.model.preparePoints('alice', 2)!;
  f.points.currencyName = 'Coins'; f.points.pointsRevision = f.status.pointsRevision = '2'; await f.model.refresh();
  expect(f.model.pointsIsCurrent(old)).toBe(false);
  f.points.currencyName = 'Gems';
  await f.model.selectPoints(' @missing ');
  expect(f.calls.find((c) => c.method === 'controller.points.select')?.params).toEqual({ epoch: '1', uniqueId: 'missing' });
  const t = f.model.preparePoints('missing', 3)!; expect(t.creates).toBe(true);
  f.lose(); expect(await f.model.adjustPoints(t)).toBe(false);
  expect(f.model.pointsCanAdjust()).toBe(false); expect(f.model.preparePoints('missing', 3)).toBeNull();
  await f.model.refresh(); await f.model.refresh();
  expect(f.calls.filter((c) => c.method === 'controller.points.command')).toHaveLength(1);
  expect(f.model.status.value.commandJournal?.pending?.commandId).toBe('native-id');
});

it('denies read-only, creator-scoped, invalid numeric and loading adjustments', async () => {
  const f = fixture(); await f.model.start();
  for (const n of [0, NaN, Infinity, -Infinity, 1e9 + 1]) expect(f.model.preparePoints('alice', n)).toBeNull();
  expect(f.model.preparePoints('hidden', 1)).toBeNull();
  f.status.hello!.capabilities = ['points.read']; await f.model.refresh(); expect(f.model.pointsCanAdjust()).toBe(false);
  f.status.hello!.resourceRestrictions = { live: ['creator'] }; await f.model.refresh();
  expect(f.model.points.value).toBeNull(); expect(f.model.pointsAllowed.value).toBe(false);
});
