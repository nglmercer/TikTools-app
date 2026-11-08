import { afterEach, expect, it } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createControllerModel, type ControllerStatus, type WorkflowRow } from './controller.ts';

const models: ReturnType<typeof createControllerModel>[] = [];
afterEach(() => { models.splice(0).forEach((m) => m.stop()); });
function fixture() {
  const status: ControllerStatus = { draftsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { hostId: 'host', principalId: 'mod', sessionId: 'pc-B', hostInstanceId: 'instance', capabilities: ['workflows.read'], features: {} } };
  let row: WorkflowRow = { graph: { id: 'welcome', name: 'Host original' }, version: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '1' } };
  let stored: { id: string; baseRevision: string; text: string; updatedAt: number } | null = null;
  let revision = '0'; let loseSave = false; let hold = false; let release: (() => void) | undefined;
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [row], next: null };
    if (method === 'controller.drafts.list') return { epoch: status.epoch, viewRevision: status.viewRevision, storeRevision: revision, rows: stored ? [{ id: stored.id, baseRevision: stored.baseRevision, updatedAt: 1, textBytes: stored.text.length }] : [], next: null };
    if (method === 'controller.drafts.get') {
      const reply = structuredClone({ epoch: status.epoch, viewRevision: status.viewRevision, storeRevision: revision, draft: stored });
      if (hold) { hold = false; return new Promise((resolve) => { release = () => resolve(reply); }); }
      return reply;
    }
    if (method === 'controller.drafts.save' || method === 'controller.drafts.delete') {
      if (params?.['expectedStoreRevision'] !== revision) throw new ControlCallError('draft_conflict', 'Review saved copy');
      revision = String(Number(revision) + 1);
      stored = method === 'controller.drafts.delete' ? null : { id: String(params?.['id']), baseRevision: String(params?.['baseRevision']), text: String(params?.['text']), updatedAt: 1 };
      if (loseSave) { loseSave = false; throw new ControlCallError('transport', 'Save not confirmed; review the saved copy'); }
      return { epoch: status.epoch, viewRevision: status.viewRevision, storeRevision: revision };
    }
    throw new Error(`Unexpected native call: ${method}`);
  });
  const model = () => { const m = createControllerModel(client); models.push(m); return m; };
  return { model, status, calls, ownerEdit: () => { status.viewRevision = '2'; row = { graph: { id: 'welcome', name: 'Host changed' }, version: { ...row.version, revision: '2' } }; },
    seed: (text: string) => { stored = { id: 'welcome', baseRevision: '1', text, updatedAt: 1 }; revision = '1'; },
    lose: () => { loseSave = true; }, hold: () => { hold = true; }, release: () => release!(), blocked: () => !!release };
}

it('stores only an explicit frozen draft, restores it on page recreation and compares/rebases without a host write', async () => {
  const f = fixture(); const m = f.model(); await m.start();
  m.edit(m.rows.value[0]!, 'incomplete local text\nfixture credential marker'); await m.draftStorage.load('welcome');
  const t = m.draftStorage.prepare('welcome')!; expect(t).toMatchObject({ revision: '1', expectedStoreRevision: '0' });
  expect(f.calls.some((c) => c.method === 'controller.drafts.save')).toBe(false);
  expect(await m.draftStorage.save(t)).toBe(true); m.stop();
  const reopened = f.model(); await reopened.start(); await reopened.draftStorage.load('welcome');
  expect(reopened.draft('welcome')).toEqual({ text: t.text, revision: '1' });
  f.ownerEdit(); await reopened.refresh();
  expect(reopened.draft('welcome')?.revision).toBe('1'); expect(reopened.rows.value[0]!.graph['name']).toBe('Host changed');
  reopened.rebaseDraft(reopened.rows.value[0]!);
  expect(reopened.draft('welcome')).toEqual({ text: t.text, revision: '2' });
  expect(reopened.draftStorage.saved('welcome')?.baseRevision).toBe('1');
  await reopened.discard('welcome'); expect(reopened.draft('welcome')).toBeUndefined();
  expect(reopened.draftStorage.saved('welcome')).toBeNull();
  expect(f.calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward' && !c.method.endsWith('.command'))).toBe(true);
});

it('keeps a newer unsaved edit when a saved-copy read arrives late and invalidates an edited confirmation', async () => {
  const f = fixture(); f.seed('older saved text'); const m = f.model(); await m.start(); f.hold();
  const loading = m.draftStorage.load('welcome');
  while (!f.blocked()) await Promise.resolve();
  m.edit(m.rows.value[0]!, 'newer unsaved edit'); f.release(); await loading;
  expect(m.draft('welcome')?.text).toBe('newer unsaved edit'); expect(m.draftStorage.saved('welcome')?.text).toBe('older saved text');
  const t = m.draftStorage.prepare('welcome')!;
  m.edit(m.rows.value[0]!, 'edit after opening the confirmation');
  expect(m.draftStorage.current(t)).toBe(false); expect(await m.draftStorage.save(t)).toBe(false);
  expect(f.calls.some((c) => c.method === 'controller.drafts.save')).toBe(false);
});

it('does not retry an unconfirmed save and requires reviewing the saved copy before further replacement', async () => {
  const f = fixture(); const m = f.model(); await m.start(); m.edit(m.rows.value[0]!, 'confirmed selection'); await m.draftStorage.load('welcome');
  const t = m.draftStorage.prepare('welcome')!; f.lose(); expect(await m.draftStorage.save(t)).toBe(false);
  expect(m.draft('welcome')?.text).toBe(t.text); expect(m.draftStorage.prepare('welcome')).toBeNull();
  expect(m.draftStorage.saved('welcome')).toBeNull();
  await m.discard('welcome'); expect(m.draft('welcome')?.text).toBe(t.text);
  expect(f.calls.some((c) => c.method === 'controller.drafts.delete')).toBe(false);
  await m.refresh(); await m.refresh(); expect(f.calls.filter((c) => c.method === 'controller.drafts.save')).toHaveLength(1);
  await m.draftStorage.load('welcome', true); expect(m.draftStorage.saved('welcome')?.text).toBe(t.text);
  expect(m.draftStorage.prepare('welcome')?.expectedStoreRevision).toBe('1');
  f.status.hello!.resourceRestrictions = { workflow: [] }; f.status.viewRevision = '2'; await m.refresh();
  expect(m.draftIds()).toEqual([]); expect(m.draftStorage.saved('welcome')).toBeNull(); expect(m.draftStorage.prepare('welcome')).toBeNull();
});

it('fences a delayed saved-copy response after changing host scope', async () => {
  const f = fixture(); f.seed('host A private text'); const m = f.model(); await m.start(); f.hold();
  const loading = m.draftStorage.load('welcome'); while (!f.blocked()) await Promise.resolve();
  f.status.epoch = '2'; f.status.hello!.hostId = 'B'; f.status.origin = 'https://B'; await m.refresh();
  f.release(); await loading;
  expect(m.draft('welcome')).toBeUndefined(); expect(m.draftStorage.saved('welcome')).toBeNull();
});

it('never carries an awaiting discard into another host scope', async () => {
  const f = fixture(); f.seed('host A private saved text'); const m = f.model(); await m.start(); f.hold();
  const removing = m.draftStorage.remove('welcome', true); while (!f.blocked()) await Promise.resolve();
  f.status.epoch = '2'; f.status.hello!.hostId = 'B'; f.status.origin = 'https://B'; await m.refresh();
  f.release(); expect(await removing).toBe(false);
  expect(f.calls.some((c) => c.method === 'controller.drafts.delete')).toBe(false);
});
