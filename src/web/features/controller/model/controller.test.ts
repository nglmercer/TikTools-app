import { afterEach, describe, expect, it } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createControllerModel, type ControllerStatus, type WorkflowRow, type CommandJournal } from './controller.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';

const models: ReturnType<typeof createControllerModel>[] = [];
afterEach(() => { models.splice(0).forEach((m) => m.stop()); });
function status(host = 'A', epoch = '1', revision = '1'): ControllerStatus {
  return { epoch, viewRevision: revision, phase: 'ready', profile: host, origin: `https://${host}`, errorCode: null,
    hello: { hostId: host, principalId: 'moderator', sessionId: host, hostInstanceId: 'instance', capabilities: [], features: { mutations: false } } };
}
function row(name = 'Host graph', revision = '1'): WorkflowRow { return { graph: { id: 'workflow', name }, version: { kind: 'workflow', id: 'workflow', scope: 'host', revision } }; }
function writable(host = 'A', epoch = '1', revision = '1'): ControllerStatus {
  const s = status(host, epoch, revision); s.hello!.capabilities = ['workflows.read', 'workflows.toggle', 'workflows.edit', 'commands.read'];
  s.hello!.features = { workflowDisable: true, workflowDelete: true };
  s.commandJournal = { binding: { profileId: host, origin: s.origin!, hostId: host, principalId: 'moderator', sessionId: host }, pending: null, recent: [], unavailable: false }; return s;
}
describe('native controller view scope', () => {
  it('keeps a deleted workflow draft accessible only while its current grant permits that resource', async () => {
    let current = writable(); let hostRows = [row()];
    const { client } = createFakeControlClient((method) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(current);
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: hostRows, next: null };
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start(); model.edit(model.rows.value[0]!, 'retained local work');
    current = writable('A', '1', '2'); hostRows = []; await model.refresh();
    expect(model.draftIds()).toEqual(['workflow']); model.editRetainedDraft('workflow', 'recovered local work');
    current.hello!.resourceRestrictions = { workflow: [] }; current.viewRevision = '3'; await model.refresh();
    expect(model.draftIds()).toEqual([]); model.editRetainedDraft('workflow', 'must not replace a hidden draft');
    current.hello!.resourceRestrictions = { workflow: ['workflow'] }; current.viewRevision = '4'; await model.refresh();
    expect(model.draftIds()).toEqual(['workflow']); expect(model.draft('workflow')?.text).toBe('recovered local work');
  });
  it('sends only the frozen command target to the native journal and preserves the local draft', async () => {
    let current = writable(); let graph = row();
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(current);
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [graph], next: null };
      if (method === 'controller.workflow.command') { current = writable('A', '1', '2'); graph = row('Authoritative host change', '2'); return {}; }
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    model.edit(model.rows.value[0]!, 'private local draft'); const target = model.prepareCommand(model.rows.value[0]!, 'disable')!;
    expect(await model.changeWorkflow(target)).toBe(true);
    expect(calls.find((c) => c.method === 'controller.workflow.command')?.params).toEqual({ epoch: '1', viewRevision: '1', id: 'workflow', expectedRevision: '1', operation: 'disable' });
    expect(calls.some((c) => c.method === 'controller.forward' || !c.method.startsWith('controller.'))).toBe(false);
    expect(model.draft('workflow')).toEqual({ text: 'private local draft', revision: '1' });
    expect(model.rows.value[0]?.version.revision).toBe('2');
  });
  it('rejects stale confirmations after a host update or profile switch before sending anything', async () => {
    let current = writable(); let graph = row();
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(current);
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [graph], next: null };
      if (method === 'controller.connect') { current = writable(String(params?.['profile']), '2'); return {}; }
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    const old = model.prepareCommand(model.rows.value[0]!, 'delete')!;
    current = writable('A', '1', '2'); graph = row('Owner changed it', '2'); await model.refresh();
    expect(await model.changeWorkflow(old)).toBe(false);
    const previousProfile = model.prepareCommand(model.rows.value[0]!, 'delete')!;
    await model.connect('B'); expect(await model.changeWorkflow(previousProfile)).toBe(false);
    expect(calls.some((c) => c.method === 'controller.workflow.command')).toBe(false);
  });
  it('blocks new changes after an unknown result and only reviews it through an explicit native action', async () => {
    const current = writable(); const graph = row();
    const pending = { commandId: 'native-generated-id', operation: 'delete' as const, resourceId: 'workflow', hostInstanceId: 'instance', expectedRevision: '1', createdAt: 1 };
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(current);
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [graph], next: null };
      if (method === 'controller.workflow.command') { current.commandJournal!.pending = pending; current.viewRevision = '2'; throw new ControlCallError('transport', 'Unconfirmed'); }
      if (method === 'controller.command.resolve') throw new ControlCallError('forbidden', 'Cannot confirm');
      if (method === 'controller.command.review') { current.commandJournal!.pending = null; current.viewRevision = '3'; return {}; }
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    expect(await model.changeWorkflow(model.prepareCommand(model.rows.value[0]!, 'delete')!)).toBe(false);
    expect(model.error.value).toBe('Unconfirmed'); expect(model.canChange('delete')).toBe(false);
    await model.refresh(); await model.refresh(); expect(calls.filter((c) => c.method === 'controller.workflow.command')).toHaveLength(1);
    expect(await model.resolveCommand()).toBe(false); expect(model.status.value.commandJournal?.pending?.commandId).toBe(pending.commandId);
    const review = model.prepareReview()!; expect(await model.reviewUnknown(review)).toBe(true);
    expect(calls.find((c) => c.method === 'controller.command.review')?.params).toEqual({ epoch: '1', commandId: pending.commandId, acknowledgeUnknown: true });
    expect(model.canChange('delete')).toBe(true); expect(calls.filter((c) => c.method === 'controller.workflow.command')).toHaveLength(1);
  });
  it('ignores a previous profile command reply and requires a journal bound to the current identity', async () => {
    let current = writable(); let release: ((value: unknown) => void) | undefined;
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return structuredClone(current);
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [row()], next: null };
      if (method === 'controller.workflow.command') return new Promise((resolve) => { release = resolve; });
      if (method === 'controller.connect') { current = writable(String(params?.['profile']), '2'); return {}; }
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    const previous = model.changeWorkflow(model.prepareCommand(model.rows.value[0]!, 'delete')!); await Promise.resolve();
    await model.connect('B'); release!({ binding: { hostId: 'A' } }); await previous;
    expect(model.status.value.hello?.hostId).toBe('B'); expect(model.error.value).toBe('');
    current.commandJournal = { ...current.commandJournal!, binding: { ...current.commandJournal!.binding, hostId: 'A' } } as CommandJournal;
    await model.refresh(); expect(model.canChange('delete')).toBe(false);
  });
  it('hides disconnected host state and keeps drafts separate from authoritative updates', async () => {
    let current = status(); let graph = row();
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.status') return { ...current };
      if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [graph], next: null };
      throw new Error(`Unexpected local/host call ${method}`);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    model.edit(model.rows.value[0]!, 'unsaved local draft');
    current = { ...current, phase: 'reconnecting' }; await model.refresh();
    expect(model.ready.value).toBe(false); expect(model.rows.value).toEqual([]);
    current = status('A', '1', '2'); graph = row('New owner edit', '2'); await model.refresh();
    expect(model.ready.value).toBe(true); expect(model.rows.value[0]?.graph['name']).toBe('New owner edit');
    expect(model.draft('workflow')).toEqual({ text: 'unsaved local draft', revision: '1' });
    expect(calls.every((c) => c.method.startsWith('controller.'))).toBe(true);
    expect(calls.some((c) => c.method === 'controller.forward')).toBe(false);
  });
  it('never applies a previous profile page after selecting another host', async () => {
    let current = status(); let delayed = false;
    let release: ((page: unknown) => void) | undefined;
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'controller.profiles') return [];
      if (method === 'controller.connect') { current = status(String(params?.['profile']), '2'); return {}; }
      if (method === 'controller.status') return { ...current };
      if (method === 'controller.workflows') {
        if (delayed) { delayed = false; return new Promise((resolve) => { release = resolve; }); }
        return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [row(current.profile!)], next: null };
      }
      throw new Error(method);
    });
    const model = createControllerModel(client); models.push(model); await model.start();
    model.edit(model.rows.value[0]!, 'A draft');
    current = status('A', '1', '2'); delayed = true;
    const refresh = model.refresh(); await Promise.resolve(); await Promise.resolve();
    expect(release).toBeTypeOf('function');
    const connect = model.connect('B');
    expect(model.rows.value).toEqual([]);
    release!({ epoch: '1', viewRevision: '2', rows: [row('A late secret')], next: null });
    await Promise.all([refresh, connect]);
    expect(model.rows.value[0]?.graph['name']).toBe('B');
    expect(model.draft('workflow')).toBeUndefined();
    await model.connect('A');
    expect(model.draft('workflow')?.text).toBe('A draft');
  });
});
