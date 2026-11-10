import { mount, flushPromises } from '@vue/test-utils';
import { expect, it } from 'vitest';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';
import type { AutomationRow } from '../model/automations.ts';
import ControllerRoot from './ControllerRoot.vue';

it.each(['en', 'es'] as const)('reviews a complete event and retains uncertain command identity (%s)', async (locale) => {
  await activateLocale(locale);
  const status: ControllerStatus = { epoch: '1', viewRevision: '1', automationRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host-A', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['automation.read', 'automation.toggle', 'automation.edit', 'commands.read'], features: { automationSummary: true, automationEventDeactivate: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host-A', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
  const rows: AutomationRow[] = ['event', 'action'].map((kind) => ({ summary: { kind: kind as 'event' | 'action', id: 'same', name: `${kind} label`, enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: kind, id: 'same', revision: '1' } }));
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.automations') return { epoch: status.epoch, automationRevision: status.automationRevision, rows, next: null };
    if (method === 'controller.automation.command') {
      status.commandJournal!.pending = { commandId: 'native-original', operation: params?.['operation'] as 'disable-event', resourceId: 'same', hostInstanceId: 'instance', expectedRevision: '1', createdAt: 1 };
      status.viewRevision = '2'; throw new ControlCallError('transport', 'Acknowledgement lost');
    }
    throw new Error(`Unexpected native call ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  const words = locale === 'es' ? { disable: 'Desactivar evento', cancel: 'Cancelar', unknown: 'las nuevas modificaciones están bloqueadas', action: 'Acción · same', revision: 'Revisión 1', started: 'Las acciones ya iniciadas pueden terminar.' } :
    { disable: 'Disable event', cancel: 'Cancel', unknown: 'new changes are blocked', action: 'Action · same', revision: 'Revision 1', started: 'Already started actions may finish.' };
  try {
    await flushPromises();
    const board = wrapper.get('.automation-board');
    expect(board.findAll('li')).toHaveLength(2); expect(board.text()).toContain(words.action);
    const actionButtons = board.findAll('li')[1]!.findAll('button');
    expect(actionButtons.every((b) => b.attributes('disabled') !== undefined)).toBe(true);
    expect(wrapper.find('textarea').exists()).toBe(false);
    expect(calls.some((c) => c.method === 'controller.automation.command')).toBe(false);
    await board.findAll('button').find((b) => b.text() === words.disable)!.trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'controller.automation.command')).toHaveLength(1);
    expect(wrapper.text()).toContain(words.unknown); expect(wrapper.get('.controller__journal').text()).toContain(words.disable);
    expect(wrapper.get('.controller__journal').text()).not.toContain(locale === 'es' ? 'Desactivar workflow' : 'Disable workflow');
    expect(wrapper.get('.controller__journal').text()).toContain('native-original');
    expect(board.findAll('button').every((b) => b.attributes('disabled') !== undefined)).toBe(true);
  } finally { wrapper.unmount(); }
});

it.each(['grant', 'read', 'connection'] as const)('removes inaccessible event labels after %s changes', async (change) => {
  await activateLocale('en');
  const { createControllerModel } = await import('../model/controller.ts');
  const { default: ControllerAutomations } = await import('./ControllerAutomations.vue');
  const status: ControllerStatus = { epoch: '1', viewRevision: '1', automationRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['automation.read', 'automation.edit'], features: { automationSummary: true, automationEventDeactivate: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
  let rows: AutomationRow[] = [{ summary: { kind: 'event', id: 'private', name: 'Captured private event label', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: 'event', id: 'private', revision: '1' } }];
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.automations') return { epoch: status.epoch, automationRevision: status.automationRevision, rows, next: null };
    throw new Error(method);
  });
  const model = createControllerModel(client); await model.start();
  const wrapper = mount(ControllerAutomations, { props: { model: model.automation, epoch: '1', busy: false }, global: { plugins: [i18n] } });
  try {
    expect(wrapper.text()).toContain('Captured private event label');
    if (change === 'grant') { status.hello!.grantRevision = '2'; status.hello!.resourceRestrictions = { automation: [] }; rows = []; }
    if (change === 'read') status.hello!.capabilities = [];
    if (change === 'connection') { status.hello!.connectionId = 'new-connection'; rows = []; }
    await model.refresh(); await flushPromises();
    expect(wrapper.find('dialog[open]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Captured private event label');
    expect(calls.some((c) => c.method === 'controller.automation.command')).toBe(false);
  } finally { wrapper.unmount(); model.stop(); }
});

it('shows enable buttons for disabled action and event when reusable activation is advertised', async () => {
  await activateLocale('en');
  const { createControllerModel } = await import('../model/controller.ts');
  const { default: ControllerAutomations } = await import('./ControllerAutomations.vue');
  const status: ControllerStatus = { epoch: '1', viewRevision: '1', automationRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['automation.read', 'automation.toggle'], features: { automationSummary: true, automationReusableActivation: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
  const rows: AutomationRow[] = ['event', 'action'].map((kind) => ({ summary: { kind: kind as 'event' | 'action', id: 'same', name: kind, enabled: false, definitionRedacted: true }, version: { kind: 'automation', scope: kind, id: 'same', revision: '1' } }));
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(status);
    if (method === 'controller.workflows') return { epoch: status.epoch, viewRevision: status.viewRevision, rows: [], next: null };
    if (method === 'controller.automations') return { epoch: status.epoch, automationRevision: status.automationRevision, rows, next: null };
    if (method === 'controller.automation.command') return {};
    throw new Error(method);
  });
  const model = createControllerModel(client); await model.start();
  const wrapper = mount(ControllerAutomations, { props: { model: model.automation, epoch: '1', busy: false }, global: { plugins: [i18n] } });
  try {
    const button = wrapper.findAll('li')[1]!.findAll('button').find((item) => item.text() === 'Enable')!;
    expect(button.attributes('disabled')).toBeUndefined();
    await button.trigger('click'); await flushPromises();
    expect(calls.find((call) => call.method === 'controller.automation.command')?.params).toMatchObject({ operation: 'enable-action', id: 'same' });
  } finally { wrapper.unmount(); model.stop(); }
});
