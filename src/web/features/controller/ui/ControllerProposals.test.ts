import { mount, flushPromises } from '@vue/test-utils';
import { expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createProposalModel, type ProposalDraft } from '../model/proposals.ts';
import type { ControllerStatus } from '../model/controller.ts';
import ControllerProposals from './ControllerProposals.vue';

it.each(['en', 'es'] as const)('reviews source storage, sends once and clears private text after permission loss (%s)', async (locale) => {
  await activateLocale(locale);
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://owner.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host-A', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } } });
  const identity = { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1' };
  let copy: ProposalDraft | null = null;
  const { client, calls } = createFakeControlClient((method, p) => {
    if (method === 'controller.proposals.list') return { ...identity, storeRevision: copy ? '1' : '0', rows: copy ? [{ ...copy, text: undefined, textBytes: copy.text.length }] : [], next: null };
    if (method === 'controller.proposals.save') { copy = { kind: 'action', id: 'same', mode: 'create', baseRevision: '0', text: String(p!['text']), updatedAt: 1 }; return { ...identity, storeRevision: '1' }; }
    if (method === 'controller.proposals.inspect') return { ...identity, kind: 'action', id: 'same', revision: '0', inspectionId: 'native-inspection', summary: null };
    throw new Error(method);
  });
  const send = vi.fn<(method: string, params: Record<string, unknown>) => Promise<boolean>>(async () => true);
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send }); await model.refresh();
  const wrapper = mount(ControllerProposals, { props: { model }, global: { plugins: [i18n] } });
  const labels = locale === 'es' ? { start: 'Preparar propuesta nueva', inspect: 'Consultar estado actual', save: 'Guardar propuesta en esta PC', send: 'Enviar propuesta para revisión', cancel: 'Cancelar', consent: 'valores sensibles' } :
    { start: 'Prepare new proposal', inspect: 'Inspect current status', save: 'Save proposal on this PC', send: 'Send proposal for review', cancel: 'Cancel', consent: 'sensitive values' };
  const button = (label: string) => wrapper.findAll('button').find((b) => b.text() === label)!;
  try {
    await wrapper.get('input').setValue('same'); await wrapper.get('form').trigger('submit');
    const text = JSON.stringify({ id: 'same', enabled: false, typeId: 'core.log', config: { message: 'private candidate source' } });
    await wrapper.get('textarea').setValue(text); await button(labels.inspect).trigger('click'); await flushPromises();
    expect(button(labels.send).attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain(labels.consent);
    await button(labels.save).trigger('click'); await flushPromises();
    expect(wrapper.find('dialog[open]').exists()).toBe(false);
    expect(calls.filter((c) => c.method === 'controller.proposals.save')).toHaveLength(1);
    await button(labels.send).trigger('click'); await flushPromises();
    expect(send).toHaveBeenCalledTimes(1);
    expect(send.mock.calls[0]?.[0]).toBe('controller.proposals.submit');
    expect(send.mock.calls[0]?.[1]).toMatchObject({ kind: 'action', id: 'same', inspectionId: 'native-inspection', acknowledgeProposal: true });
    status.value.hello!.capabilities = []; await flushPromises();
    expect(wrapper.find('dialog[open]').exists()).toBe(false); expect(wrapper.text()).not.toContain('private candidate source'); expect(wrapper.find('textarea').exists()).toBe(false); expect(model.text.value).toBe(''); expect(send).toHaveBeenCalledTimes(1);
  } finally { wrapper.unmount(); model.stop(); }
});

it('applies a saved active definition without a repeated approval dialog', async () => {
  await activateLocale('en');
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://owner.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit', 'automation.toggle'], features: { automationDrafts: true, automationReusableWrite: true } } });
  const identity = { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1' };
  const { client } = createFakeControlClient((method) => {
    if (method === 'controller.proposals.list') return { ...identity, storeRevision: '0', rows: [], next: null };
    if (method === 'controller.proposals.inspect') return { ...identity, kind: 'action', id: 'same', revision: '0', inspectionId: 'inspection', summary: null };
    if (method === 'controller.proposals.save') return { ...identity, storeRevision: '1' };
    throw new Error(method);
  });
  const send = vi.fn(async () => true); const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send });
  await model.refresh(); model.select('action', 'same'); await model.inspect(); await model.confirm(model.prepare('save')!);
  const wrapper = mount(ControllerProposals, { props: { model }, global: { plugins: [i18n] } });
  try {
    await wrapper.findAll('button').find((button) => button.text() === 'Apply active')!.trigger('click'); await flushPromises();
    expect(send).toHaveBeenCalledExactlyOnceWith('controller.proposals.submit', expect.objectContaining({ active: true, kind: 'action', id: 'same' }));
    expect(wrapper.find('dialog[open]').exists()).toBe(false);
  } finally { wrapper.unmount(); model.stop(); }
});

it('previews explicitly shared source before replacing the open editor', async () => {
  await activateLocale('en');
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://owner.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true, automationSharedSource: true } } });
  const identity = { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1' };
  const { client } = createFakeControlClient((method) => {
    if (method === 'controller.proposals.list') return { ...identity, storeRevision: '0', rows: [], next: null };
    if (method === 'controller.proposals.source') return { ...identity, kind: 'action', id: 'same', revision: '2', record: { id: 'same', name: 'Owner definition', enabled: true, typeId: 'core.log', config: { message: 'shared value' } } };
    throw new Error(method);
  });
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send: async () => true }); await model.refresh(); model.select('action', 'same');
  const before = model.text.value; const wrapper = mount(ControllerProposals, { props: { model }, global: { plugins: [i18n] } });
  try {
    await wrapper.findAll('button').find((button) => button.text() === 'Read shared definition')!.trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('shared value'); expect(model.text.value).toBe(before);
    await wrapper.findAll('button').find((button) => button.text() === 'Use this definition')!.trigger('click');
    expect(JSON.parse(model.text.value)).toMatchObject({ name: 'Owner definition', enabled: false });
    expect(model.baseRevision.value).toBe('2'); expect(model.mode.value).toBe('update');
    const builder = wrapper.get('.proposal-builder');
    await builder.findAll('button').find((button) => button.text() === 'Load fields from open text')!.trigger('click');
    expect((builder.findAll('input')[1]!.element as HTMLInputElement).value).toBe('shared value');
    await builder.findAll('input')[1]!.setValue('changed through fields');
    await builder.findAll('button').find((button) => button.text() === 'Prepare draft')!.trigger('click');
    await wrapper.get('dialog[open]').findAll('button').find((button) => button.text() === 'Use this draft')!.trigger('click');
    expect(JSON.parse(model.text.value)).toMatchObject({ name: 'Owner definition', enabled: false, config: { message: 'changed through fields' } });
    expect(model.baseRevision.value).toBe('2');
  } finally { wrapper.unmount(); model.stop(); }
});
