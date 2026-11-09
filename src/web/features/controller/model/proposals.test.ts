import { afterEach, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { createProposalModel, type ProposalDraft } from './proposals.ts';
import type { ControllerStatus } from './controller.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
const models: ReturnType<typeof createProposalModel>[] = [];
afterEach(() => { for (const model of models.splice(0)) model.stop(); });
function fixture() {
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } } });
  let storeRevision = 0; let hostRevision = '0'; let copy: ProposalDraft | null = null; let delayed: Promise<unknown> | undefined; let delayedSave: Promise<unknown> | undefined;
  const identity = () => ({ epoch: status.value.epoch, connectionId: status.value.hello!.connectionId, hostInstanceId: status.value.hello!.hostInstanceId, grantRevision: status.value.hello!.grantRevision });
  const { client, calls } = createFakeControlClient((method, p) => {
    if (method === 'controller.proposals.list') return { ...identity(), storeRevision: String(storeRevision), rows: copy ? [{ kind: copy.kind, id: copy.id, mode: copy.mode, baseRevision: copy.baseRevision, updatedAt: 1, textBytes: copy.text.length }] : [], next: null };
    if (method === 'controller.proposals.get') return delayed ?? { ...identity(), storeRevision: String(storeRevision), draft: copy };
    if (method === 'controller.proposals.source') return { ...identity(), kind: p!['kind'], id: p!['id'], revision: '2', record: { id: p!['id'], name: 'Owner source', enabled: true, typeId: 'core.log', config: { message: 'shared sensitive value' } } };
    if (method === 'controller.proposals.inspect') return { ...identity(), inspectionId: `inspection-${hostRevision}`, kind: p!['kind'], id: p!['id'], revision: hostRevision, summary: hostRevision === '0' ? null : { kind: p!['kind'], id: p!['id'], name: 'Closed host summary', enabled: false, definitionRedacted: true } };
    if (method === 'controller.proposals.save') { if (delayedSave) return delayedSave; copy = { kind: p!['kind'] as 'action', id: String(p!['id']), mode: p!['mode'] as 'create', baseRevision: String(p!['baseRevision']), text: String(p!['text']), updatedAt: 1 }; storeRevision += 1; return { ...identity(), storeRevision: String(storeRevision) }; }
    if (method === 'controller.proposals.delete') { copy = null; storeRevision += 1; return { ...identity(), storeRevision: String(storeRevision) }; }
    throw new Error(method);
  });
  let blocked = false;
  const send = vi.fn(async () => true);
  const model = createProposalModel(client, { status, blocked: () => blocked, journalMatches: () => true, send }); models.push(model);
  return { model, status, calls, send, identity, block: () => { blocked = true; }, host: (revision: string) => { hostRevision = revision; }, changeCopy: (draft: ProposalDraft) => { copy = draft; storeRevision += 1; }, delay: (p: Promise<unknown>) => { delayed = p; }, delaySave: (p: Promise<unknown>) => { delayedSave = p; } };
}

it('does not send after the grant changes while saving', async () => {
  const f = fixture(); await f.model.refresh(); f.model.select('action', 'same'); await f.model.inspect();
  const oldIdentity = f.identity(); let release: (value: unknown) => void = () => {};
  f.delaySave(new Promise((resolve) => { release = resolve; }));
  const submitting = f.model.submit(false);
  f.status.value.hello!.grantRevision = '2';
  release({ ...oldIdentity, storeRevision: '1' });
  await submitting;
  expect(f.send).not.toHaveBeenCalled();
});

it('inspects, saves and submits in one action without silently replacing a newer host revision', async () => {
  const f = fixture(); await f.model.refresh(); f.model.select('action', 'same');
  await f.model.submit(false);
  expect(f.calls.filter((call) => call.method === 'controller.proposals.inspect')).toHaveLength(1);
  expect(f.calls.filter((call) => call.method === 'controller.proposals.save')).toHaveLength(1);
  expect(f.send).toHaveBeenCalledTimes(1);
  f.host('2'); f.model.edit(`${f.model.text.value}\n`);
  await f.model.submit(false);
  expect(f.model.error.value).toBe('proposal_base_changed');
  expect(f.send).toHaveBeenCalledTimes(1);
});
async function ready(f: ReturnType<typeof fixture>) {
  await f.model.refresh(); f.model.select('action', 'same'); await f.model.inspect();
  const text = JSON.stringify({ id: 'same', enabled: false, typeId: 'core.log', config: { message: 'local secret' } }); f.model.edit(text);
  await f.model.confirm(f.model.prepare('save')!); return text;
}

it('requires an explicit saved copy and inspection, sends only a native reference and never turns a summary into a definition', async () => {
  const f = fixture(); await f.model.refresh(); f.model.select('action', 'same');
  expect(f.model.text.value).toContain('typeId'); expect(f.model.prepare('send')).toBeNull();
  await f.model.inspect(); expect(f.model.prepare('send')).toBeNull();
  await f.model.confirm(f.model.prepare('save')!);
  const captured = f.model.prepare('send')!; expect(captured).not.toBeNull();
  await f.model.confirm(captured);
  expect(f.send).toHaveBeenCalledExactlyOnceWith('controller.proposals.submit', { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', kind: 'action', id: 'same', inspectionId: 'inspection-0', expectedStoreRevision: '1', acknowledgeProposal: true });
  expect(f.model.inspection.value).toBeNull(); expect(f.model.prepare('send')).toBeNull();
  f.host('1'); await f.model.inspect(); expect(f.model.text.value).toBe(captured.text); expect(f.model.text.value).not.toContain('Closed host summary');
  expect(f.model.baseRevision.value).toBe('0'); expect(f.model.mode.value).toBe('create');
  await f.model.confirm(f.model.prepare('base')!); expect(f.model.baseRevision.value).toBe('1'); expect(f.model.mode.value).toBe('update'); expect(f.model.prepare('send')).toBeNull();
  await f.model.confirm(f.model.prepare('save')!); expect(f.model.prepare('send')?.baseRevision).toBe('1');
  expect(f.calls.every((c) => c.method.startsWith('controller.proposals.'))).toBe(true);
});

it('invalidates confirmations on local edits, preserves unsaved text on catalog changes and never silently rebases a saved copy', async () => {
  const f = fixture(); const text = await ready(f); const captured = f.model.prepare('send')!;
  f.model.edit(`${text}\n`); expect(f.model.current(captured)).toBe(false); await f.model.confirm(captured); expect(f.send).not.toHaveBeenCalled();
  f.changeCopy({ kind: 'action', id: 'same', mode: 'create', baseRevision: '0', text: 'another saved copy', updatedAt: 1 });
  await f.model.reloadInventory(); expect(f.model.text.value).toBe(`${text}\n`); expect(f.model.storeRevision.value).toBe('1'); expect(f.model.error.value).toBe('proposal_conflict');
  await f.model.load(); expect(f.model.text.value).toBe('another saved copy'); expect(f.model.storeRevision.value).toBe('2'); expect(f.model.inspection.value).toBeNull();
});

it.each(['connectionId', 'grantRevision', 'hostId', 'hostInstanceId', 'principalId', 'sessionId'] as const)('scrubs text and captures immediately after %s changes', async (field) => {
  const f = fixture(); await ready(f); const captured = f.model.prepare('send')!;
  f.status.value.hello![field] = 'changed'; expect(f.model.text.value).toBe(''); expect(f.model.saved.value).toBeNull(); expect(f.model.inspection.value).toBeNull(); expect(f.model.current(captured)).toBe(false);
  await f.model.confirm(captured); expect(f.send).not.toHaveBeenCalled();
});

it('shares the pending gate while read only access can retain text and live restrictions hide it', async () => {
  const f = fixture(); await ready(f); f.block(); expect(f.model.canSend.value).toBe(false); expect(f.model.prepare('save')).not.toBeNull();
  f.status.value.hello!.capabilities = ['automation.read']; await f.model.refresh(); f.model.select('event', 'same'); await f.model.inspect();
  expect(f.model.prepare('save')).not.toBeNull(); expect(f.model.prepare('send')).toBeNull();
  f.status.value.hello!.resourceRestrictions = { automation: ['action:same'] }; f.model.select('event', 'same'); expect(f.model.editable.value).toBe(false);
  f.status.value.hello!.resourceRestrictions = { live: ['creator'] }; expect(f.model.allowed.value).toBe(false); expect(f.model.text.value).toBe('');
});

it('ignores an older selected private read after profile loss', async () => {
  const f = fixture(); await ready(f); const oldIdentity = f.identity();
  let release: (v: unknown) => void = () => {}; f.delay(new Promise((resolve) => { release = resolve; }));
  const loading = f.model.load(); f.status.value.phase = 'reconnecting';
  release({ ...oldIdentity, storeRevision: '1', draft: { kind: 'action', id: 'same', mode: 'create', baseRevision: '0', text: 'delayed secret', updatedAt: 1 } });
  await loading; expect(f.model.text.value).toBe(''); expect(f.model.saved.value).toBeNull(); expect(f.model.busy.value).toBe(false);
});

it('sends an active change only when the host advertises the reusable grant', async () => {
  const f = fixture(); await ready(f);
  expect(f.model.prepare('send-active')).toBeNull();
  f.status.value.hello!.capabilities.push('automation.toggle');
  f.status.value.hello!.features['automationReusableWrite'] = true;
  // Changing grant capabilities clears the editor; inspect and save again.
  await ready(f);
  const target = f.model.prepare('send-active')!;
  expect(target).not.toBeNull();
  await f.model.confirm(target);
  expect(f.send).toHaveBeenCalledExactlyOnceWith('controller.proposals.submit', expect.objectContaining({ kind: 'action', id: 'same', active: true, expectedStoreRevision: '2' }));
  expect(f.model.prepare('send-active')).toBeNull();
});


it('previews shared source without overwriting new edits and clears it when access changes', async () => {
  const f = fixture(); await ready(f);
  expect(f.model.canReadShared.value).toBe(false);
  f.status.value.hello!.features['automationSharedSource'] = true;
  await f.model.loadShared();
  expect(f.model.sharedSource.value?.text).toContain('shared sensitive value');
  const before = f.model.text.value; f.model.edit(`${before} `); f.model.useShared();
  expect(f.model.text.value).toBe(`${before} `);
  await f.model.loadShared(); f.model.useShared();
  expect(JSON.parse(f.model.text.value)).toMatchObject({ name: 'Owner source', enabled: false });
  expect(f.model.baseRevision.value).toBe('2'); expect(f.model.mode.value).toBe('update');
  await f.model.loadShared(); f.status.value.hello!.grantRevision = '2';
  expect(f.model.sharedSource.value).toBeNull(); expect(f.model.text.value).toBe('');
});
