import { mount, flushPromises } from '@vue/test-utils';
import { expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createProposalModel, type ProposalDraft } from '../model/proposals.ts';
import type { ControllerStatus } from '../model/controller.ts';
import ControllerProposals from './ControllerProposals.vue';

it.each(['en', 'es'] as const)('reviews explicit source storage and sending and removes captures after permission loss (%s)', async (locale) => {
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
  const send = vi.fn(async () => true);
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send }); await model.refresh();
  const wrapper = mount(ControllerProposals, { props: { model }, global: { plugins: [i18n] } });
  const labels = locale === 'es' ? { start: 'Preparar propuesta nueva', inspect: 'Consultar estado actual', save: 'Guardar propuesta en esta PC', send: 'Enviar propuesta para revisión', cancel: 'Cancelar', consent: 'valores sensibles' } :
    { start: 'Prepare new proposal', inspect: 'Inspect current status', save: 'Save proposal on this PC', send: 'Send proposal for review', cancel: 'Cancel', consent: 'sensitive values' };
  const button = (label: string) => wrapper.findAll('button').find((b) => b.text() === label)!;
  const dialogButton = (label: string) => wrapper.get('dialog[open]').findAll('button').find((b) => b.text() === label)!;
  try {
    await wrapper.get('input').setValue('same'); await wrapper.get('form').trigger('submit');
    const text = JSON.stringify({ id: 'same', enabled: false, typeId: 'core.log', config: { message: 'private candidate source' } });
    await wrapper.get('textarea').setValue(text); await button(labels.inspect).trigger('click'); await flushPromises();
    expect(button(labels.send).attributes('disabled')).toBeDefined();
    await button(labels.save).trigger('click');
    expect(wrapper.get('dialog[open]').text()).toContain(labels.consent); expect(wrapper.get('dialog[open]').text()).toContain('private candidate source');
    expect(calls.filter((c) => c.method === 'controller.proposals.save')).toHaveLength(0);
    await dialogButton(labels.cancel).trigger('click'); await button(labels.save).trigger('click'); await dialogButton(labels.save).trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'controller.proposals.save')).toHaveLength(1);
    await button(labels.send).trigger('click'); expect(wrapper.get('dialog[open]').text()).toContain('https://owner.example'); expect(wrapper.get('dialog[open]').text()).toContain('host-A'); expect(wrapper.get('dialog[open]').text()).toContain(text);
    await dialogButton(labels.cancel).trigger('click'); expect(send).not.toHaveBeenCalled();
    await button(labels.send).trigger('click'); status.value.hello!.capabilities = []; await flushPromises();
    expect(wrapper.find('dialog[open]').exists()).toBe(false); expect(wrapper.text()).not.toContain('private candidate source'); expect(wrapper.find('textarea').exists()).toBe(false); expect(model.text.value).toBe(''); expect(send).not.toHaveBeenCalled();
  } finally { wrapper.unmount(); model.stop(); }
});
