import { afterEach, expect, it, vi } from 'vitest';
import { createControllerModel, type ControllerStatus } from './controller.ts';
import type { AutomationRow } from './automations.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
const models: ReturnType<typeof createControllerModel>[] = [];
afterEach(() => { for (const m of models.splice(0)) m.stop(); });
function automationStatus(): ControllerStatus {
  return { epoch: '1', viewRevision: '1', automationRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host', hostInstanceId: 'instance', principalId: 'moderator', sessionId: 'device',
      capabilities: ['automation.read', 'automation.toggle', 'automation.edit', 'commands.read'], features: { automationSummary: true, automationEventDeactivate: true } },
    commandJournal: { binding: { profileId: 'profile-id', origin: 'https://host.example', hostId: 'host', principalId: 'moderator', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
}
function automationRow(kind: 'event' | 'action', revision = '1'): AutomationRow {
  return { summary: { kind, id: 'same', name: kind === 'event' ? 'Welcome event' : 'Welcome action', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: kind, id: 'same', revision } };
}
function fixture() {
  const status = automationStatus(); let rows = [automationRow('event'), automationRow('action')]; let lost = false;
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.automations') return { epoch: status.epoch, automationRevision: status.automationRevision, rows: structuredClone(rows), next: null };
    if (method === 'controller.automation.command') {
      if (lost) {
        status.commandJournal!.pending = { commandId: 'native-original', operation: params?.['operation'] as 'disable-event', resourceId: 'same', hostInstanceId: 'instance', expectedRevision: '1', createdAt: 1 };
        status.viewRevision = '2'; throw new ControlCallError('transport', 'Acknowledgement lost');
      }
      return {};
    }
    if (method === 'controller.command.resolve') return {};
    throw new Error(`Unexpected native method ${method}`);
  });
  const model = createControllerModel(client); models.push(model);
  return { model, status, calls, setRows: (value: AutomationRow[]) => { rows = value; }, lose: () => { lost = true; } };
}

it('keeps equal family IDs separate and confirms only an event with native-generated durable identity', async () => {
  const f = fixture(); await f.model.start();
  expect(f.model.automation.rows.value.map((r) => r.summary.kind)).toEqual(['event', 'action']);
  expect(f.model.automation.prepare(f.model.automation.rows.value[1]!, 'delete-event')).toBeNull();
  const target = f.model.automation.prepare(f.model.automation.rows.value[0]!, 'disable-event')!;
  expect(target).toMatchObject({ name: 'Welcome event', id: 'same', expectedRevision: '1', automationRevision: '1' });
  expect(f.calls.some((c) => c.method.includes('.command'))).toBe(false);
  expect(await f.model.automation.change(target)).toBe(true);
  expect(f.calls.filter((c) => c.method === 'controller.automation.command').map((c) => c.params)).toEqual([
    { epoch: '1', automationRevision: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', id: 'same', expectedRevision: '1', operation: 'disable-event' },
  ]);
  expect(f.calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward')).toBe(true);
});

it('enables only a disabled item of the matching family under a reusable grant', async () => {
  const f = fixture();
  f.status.hello!.features['automationReusableActivation'] = true;
  const event = automationRow('event'); const action = automationRow('action');
  event.summary.enabled = false; action.summary.enabled = false;
  f.setRows([event, action]); await f.model.start();
  expect(f.model.automation.prepare(f.model.automation.rows.value[0]!, 'enable-action')).toBeNull();
  expect(f.model.automation.prepare(f.model.automation.rows.value[1]!, 'enable-event')).toBeNull();
  const target = f.model.automation.prepare(f.model.automation.rows.value[1]!, 'enable-action')!;
  expect(await f.model.automation.change(target)).toBe(true);
  expect(f.calls.find((call) => call.method === 'controller.automation.command')?.params).toMatchObject({ operation: 'enable-action', id: 'same', expectedRevision: '1' });
  f.status.hello!.features['automationReusableActivation'] = false; f.status.hello!.grantRevision = '2'; await f.model.refresh();
  expect(f.model.automation.current(target)).toBe(false);
  expect(f.model.automation.prepare(f.model.automation.rows.value[0]!, 'enable-event')).toBeNull();
});

it('preserves event confirmation through unrelated workflow/points display revisions but fences its event update', async () => {
  const f = fixture(); await f.model.start(); const target = f.model.automation.prepare(f.model.automation.rows.value[0]!, 'delete-event')!;
  f.status.viewRevision = '2'; f.status.pointsRevision = '99'; await f.model.refresh();
  expect(f.model.automation.current(target)).toBe(true);
  expect(f.calls.filter((c) => c.method === 'controller.automations')).toHaveLength(1);
  f.status.automationRevision = '2'; f.setRows([automationRow('event', '2'), automationRow('action')]); await f.model.refresh();
  expect(f.model.automation.current(target)).toBe(false); expect(await f.model.automation.change(target)).toBe(false);
  expect(f.calls.some((c) => c.method === 'controller.automation.command')).toBe(false);
});

it.each(['connectionId', 'grantRevision', 'hostId', 'hostInstanceId', 'principalId', 'sessionId'] as const)('fences an event confirmation after %s changes', async (field) => {
  const f = fixture(); await f.model.start(); const target = f.model.automation.prepare(f.model.automation.rows.value[0]!, 'delete-event')!;
  f.status.hello![field] = 'changed'; await f.model.refresh();
  expect(f.model.automation.current(target)).toBe(false); expect(await f.model.automation.change(target)).toBe(false);
  expect(f.calls.some((c) => c.method === 'controller.automation.command')).toBe(false);
});

it('clears disconnected or creator-restricted rows and blocks reads/writes under a narrowed grant', async () => {
  const f = fixture(); await f.model.start(); const target = f.model.automation.prepare(f.model.automation.rows.value[0]!, 'delete-event')!;
  f.status.hello!.resourceRestrictions = { automation: ['action:same'] }; f.status.hello!.grantRevision = '2'; f.setRows([automationRow('action')]); await f.model.refresh();
  expect(f.model.automation.rows.value.map((r) => r.summary.kind)).toEqual(['action']); expect(f.model.automation.current(target)).toBe(false);
  f.status.hello!.resourceRestrictions = { live: ['creator'] }; await f.model.refresh();
  expect(f.model.automation.allowed.value).toBe(false); expect(f.model.automation.rows.value).toEqual([]);
  f.status.hello!.resourceRestrictions = {}; f.status.phase = 'reconnecting'; await f.model.refresh();
  expect(f.model.automation.rows.value).toEqual([]); expect(f.model.automation.current(target)).toBe(false);
});

it.each(['payload', 'flag', 'scope', 'duplicate', 'revision', 'name'] as const)('rejects malformed closed automation status: %s', async (change) => {
  const f = fixture(); const rows = [automationRow('event'), automationRow('action')];
  if (change === 'payload') Object.assign(rows[0]!.summary, { config: { secret: 'must-not-retain' } });
  if (change === 'flag') Object.assign(rows[0]!.summary, { definitionRedacted: false });
  if (change === 'scope') rows[0]!.version.scope = 'action';
  if (change === 'duplicate') rows[1] = automationRow('event');
  if (change === 'revision') rows[0]!.version.revision = '01';
  if (change === 'name') rows[0]!.summary.name = 'x'.repeat(161);
  f.setRows(rows); await f.model.start();
  expect(f.model.automation.rows.value).toEqual([]); expect(f.model.automation.ready.value).toBe(false); expect(f.model.automation.error.value).not.toBe('');
  expect(f.calls.some((c) => c.method === 'controller.automation.command')).toBe(false);
});

it('keeps uncertain event intents on their original ID and blocks all new changes without automatic replay', async () => {
  const f = fixture(); await f.model.start(); const target = f.model.automation.prepare(f.model.automation.rows.value[0]!, 'delete-event')!;
  f.lose(); expect(await f.model.automation.change(target)).toBe(false);
  expect(f.model.status.value.commandJournal?.pending?.commandId).toBe('native-original');
  expect(f.model.automation.prepare(f.model.automation.rows.value[0]!, 'delete-event')).toBeNull();
  await f.model.refresh(); await f.model.refresh();
  expect(f.calls.filter((c) => c.method === 'controller.automation.command')).toHaveLength(1);
  expect(await f.model.resolveCommand()).toBe(true);
  expect(f.calls.find((c) => c.method === 'controller.command.resolve')?.params).toEqual({ epoch: '1', commandId: 'native-original' });
});

it('rejects duplicate families across native pages and non-advancing pagination', async () => {
  for (const repeatsOffset of [false, true]) {
    const status = automationStatus();
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(status);
      if (method === 'controller.workflows') return { epoch: '1', viewRevision: '1', rows: [], next: null };
      if (method === 'controller.automations') return { epoch: '1', automationRevision: '1', rows: [automationRow('event')], next: params?.['offset'] === 0 ? (repeatsOffset ? 0 : 1) : null };
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    expect(model.automation.rows.value).toEqual([]); expect(model.automation.ready.value).toBe(false); expect(model.automation.error.value).not.toBe('');
  }
});

it('never publishes a delayed automation page from a previous profile', async () => {
  let status = automationStatus(); let release: (() => void) | undefined;
  const { client } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.connect') {
      status = { ...automationStatus(), epoch: '2', profile: 'other', origin: 'https://other.example' };
      status.hello!.hostId = 'other-host'; return {};
    }
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.automations') {
      if (params?.['epoch'] === '1') return new Promise((resolve) => { release = () => resolve({ epoch: '1', automationRevision: '1', rows: [automationRow('event')], next: null }); });
      return { epoch: '2', automationRevision: '1', rows: [], next: null };
    }
    throw new Error(method);
  });
  const model = createControllerModel(client); models.push(model); const start = model.start();
  await vi.waitFor(() => expect(release).toBeTypeOf('function'));
  const connect = model.connect('other');
  expect(model.automation.rows.value).toEqual([]); release!(); await Promise.all([start, connect]);
  expect(model.status.value.profile).toBe('other'); expect(model.automation.rows.value).toEqual([]); expect(model.automation.ready.value).toBe(true);
});
