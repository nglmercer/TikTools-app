import { mount, flushPromises } from '@vue/test-utils';
import { expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import ControllerRoot from './ControllerRoot.vue';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';

it('explains a snapshot rate budget failure without retrying a connection', async () => {
  i18n.global.locale.value = 'es';
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return { epoch: '1', viewRevision: '1', phase: 'failed', profile: 'desk', origin: 'https://host.example', hello: null, errorCode: 'snapshot_rate_budget' };
    throw new Error(`Unexpected native call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  try {
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('reduzca los recursos permitidos');
    expect(wrapper.text()).toContain('aumente el límite de solicitudes o el tamaño de mensaje');
    expect(calls.some((call) => call.method === 'controller.connect')).toBe(false);
    i18n.global.locale.value = 'en';
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('narrow the permitted resources');
  } finally { wrapper.unmount(); }
});

it('requires confirmation before forgetting a reviewed local profile', async () => {
  i18n.global.locale.value = 'en';
  let profiles = [{ name: 'desk', profileId: 'instance-1', origin: 'https://host.example', hostId: 'host-A' }];
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return structuredClone(profiles);
    if (method === 'controller.status') return { epoch: '0', viewRevision: '0', phase: 'idle', profile: null, origin: null, hello: null, errorCode: null };
    if (method === 'controller.profile.forget') { profiles = []; return { ok: true }; }
    throw new Error(`Unexpected native call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  const button = (name: string) => wrapper.findAll('button').find((node) => node.text() === name)!;
  try {
    await flushPromises();
    await button('Forget profile').trigger('click');
    const dialog = wrapper.get('dialog[open]');
    expect(dialog.text()).toContain('credentials, drafts and command history');
    expect(dialog.text()).toContain('Existing connections may remain active until they expire or the owner revokes host access');
    expect(dialog.text()).toContain('https://host.example');
    await dialog.findAll('button').find((node) => node.text() === 'Cancel')!.trigger('click');
    expect(calls.some((call) => call.method === 'controller.profile.forget')).toBe(false);
    await button('Forget profile').trigger('click');
    await wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === 'Forget profile')!.trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'controller.profile.forget')?.params).toEqual({ profile: 'desk', profileId: 'instance-1', origin: 'https://host.example', hostId: 'host-A', confirm: true });
    expect(wrapper.text()).not.toContain('https://host.example');
  } finally { wrapper.unmount(); }
});

it('renders requested points separately from verified results and includes the unit in unknown review', async () => {
  i18n.global.locale.value = 'en';
  const intent = { commandId: 'pending-points', operation: 'adjust-points' as const, resourceId: 'alice', hostInstanceId: 'instance', delta: -3, currencyName: 'Gems', createdAt: 1 };
  const current: ControllerStatus = { epoch: '1', viewRevision: '1', phase: 'ready', profile: 'points', origin: 'https://host.example', errorCode: null,
    hello: { hostId: 'host-A', principalId: 'moderator', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['points.read', 'points.adjust', 'commands.read'], features: { pointsAdjust: true } },
    commandJournal: { binding: { profileId: 'points', origin: 'https://host.example', hostId: 'host-A', principalId: 'moderator', sessionId: 'device' }, pending: intent,
      unavailable: false, recent: [{ intent: { ...intent, commandId: 'completed-points', delta: 2 }, outcome: 'succeeded', revision: '45', errorCode: null, recordedAt: 2,
        pointsResult: { delta: 2.5, totalPoints: 7.5, level: 1 } }] } };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(current);
    if (method === 'controller.workflows') return { epoch: '1', viewRevision: '1', rows: [], next: null };
    throw new Error(`Unexpected native call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  try {
    await flushPromises();
    expect(wrapper.text()).toContain('Requested amount: -3 Gems');
    expect(wrapper.text()).toContain('Requested amount: 2 Gems');
    expect(wrapper.text()).toContain('Applied adjustment: 2.5 Gems');
    expect(wrapper.text()).toContain('Confirmed balance: 7.5 Gems');
    await wrapper.findAll('button').find((node) => node.text() === 'Review unknown outcome')!.trigger('click');
    await flushPromises();
    expect(wrapper.get('dialog[open]').text()).toContain('alice');
    expect(wrapper.get('dialog[open]').text()).toContain('Requested amount: -3 Gems');
    expect(wrapper.get('dialog[open]').text()).toContain('host-A');
    expect(calls.every((call) => !call.method.includes('.command') && call.method !== 'controller.forward')).toBe(true);
  } finally { wrapper.unmount(); }
});

it('starts in controller scope and clears the private invitation after submission', async () => {
  i18n.global.locale.value = 'en';
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return { epoch: '0', viewRevision: '0', phase: 'idle', profile: null, origin: null, hello: null, errorCode: null };
    if (method === 'controller.pair') return { requestId: 'owner-approval-request' };
    if (method === 'controller.pair.poll') return { approved: false };
    throw new Error(`Unexpected owner call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  try {
    await flushPromises();
    expect(wrapper.text()).toContain('Remote controller');
    const inputs = wrapper.findAll('input');
    await inputs[0]!.setValue('https://host.example');
    await inputs[1]!.setValue('moderator');
    await inputs[3]!.setValue('private-invitation-marker');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect((inputs[3]!.element as HTMLInputElement).value).toBe('');
    expect(wrapper.text()).toContain('owner-approval-request');
    expect(calls.find((c) => c.method === 'controller.pair')?.params?.['invitation']).toBe('private-invitation-marker');
    expect(calls.every((c) => c.method.startsWith('controller.'))).toBe(true);
  } finally { wrapper.unmount(); }
});

it('requires a host-bound confirmation and blocks changes until an unknown command is explicitly reviewed', async () => {
  i18n.global.locale.value = 'en';
  const current: ControllerStatus = { epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { hostId: 'host-A', principalId: 'moderator', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['workflows.read', 'workflows.toggle', 'workflows.edit', 'commands.read'], features: { workflowDisable: true, workflowDelete: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host-A', principalId: 'moderator', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
  const intent = { commandId: 'native-command-id', operation: 'delete' as const, resourceId: 'welcome', hostInstanceId: 'instance', expectedRevision: '1', createdAt: 1 };
  const graph = { graph: { id: 'welcome', name: 'Owner workflow', enabled: true }, version: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '1' } };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(current);
    if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [graph], next: null };
    if (method === 'controller.workflow.command') { current.commandJournal!.pending = intent; current.viewRevision = '2'; throw new ControlCallError('transport', 'Unconfirmed'); }
    if (method === 'controller.command.review') { current.commandJournal!.pending = null; current.commandJournal!.recent = [{ intent, outcome: 'reviewed-unknown', revision: null, errorCode: 'command_unknown', recordedAt: 2 }]; current.viewRevision = '3'; return {}; }
    throw new Error(`Unexpected native call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  const button = (name: string) => wrapper.findAll('button').find((node) => node.text() === name)!;
  try {
    await flushPromises(); await button('Owner workflow · 1').trigger('click');
    await wrapper.get('textarea').setValue('private local draft');
    await button('Disable workflow').trigger('click'); await flushPromises();
    expect(wrapper.get('dialog[open]').text()).toContain('https://host.example');
    expect(wrapper.get('dialog[open]').text()).toContain('host-A');
    await wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === 'Cancel')!.trigger('click'); await flushPromises();
    expect(calls.some((c) => c.method === 'controller.workflow.command')).toBe(false);
    await button('Delete workflow').trigger('click'); await flushPromises();
    await wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === 'Delete workflow')!.trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'controller.workflow.command')).toHaveLength(1);
    expect(calls.find((c) => c.method === 'controller.workflow.command')?.params).toEqual({ epoch: '1', viewRevision: '1', id: 'welcome', expectedRevision: '1', operation: 'delete' });
    expect(button('Delete workflow').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('new changes are blocked');
    expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('private local draft');
    await button('Review unknown outcome').trigger('click'); await flushPromises();
    expect(wrapper.get('dialog[open]').text()).toContain('does not undo the change or prove it failed');
    await wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === 'Keep outcome unknown')!.trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Reviewed unknown outcome');
    expect(calls.find((c) => c.method === 'controller.command.review')?.params?.['acknowledgeUnknown']).toBe(true);
    expect(calls.filter((c) => c.method === 'controller.workflow.command')).toHaveLength(1);
    expect(calls.some((c) => c.method === 'controller.forward' || !c.method.startsWith('controller.'))).toBe(false);
  } finally { wrapper.unmount(); }
});
