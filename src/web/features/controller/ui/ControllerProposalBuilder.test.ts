import { mount, flushPromises } from '@vue/test-utils';
import { expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createProposalModel } from '../model/proposals.ts';
import type { ControllerStatus } from '../model/controller.ts';
import ControllerProposalBuilder from './ControllerProposalBuilder.vue';

async function setup(kind: 'action' | 'event') {
  await activateLocale('en');
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } } });
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.proposals.list') return { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', storeRevision: '0', rows: [], next: null };
    throw new Error(`unexpected ${method}`);
  });
  const send = vi.fn(async () => true);
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send });
  await model.refresh(); model.select(kind, 'author.example');
  const wrapper = mount(ControllerProposalBuilder, { props: { model }, global: { plugins: [i18n] } });
  const button = (label: string) => wrapper.findAll('button').find((node) => node.text() === label)!;
  const dialogButton = (label: string) => wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === label)!;
  return { status, model, wrapper, button, dialogButton, calls, send };
}

it('creates a disabled action only after preview, retaining the open text on cancel', async () => {
  const f = await setup('action');
  try {
    const original = JSON.stringify({ id: 'author.example', enabled: false, extra: { keep: 'private data' } });
    f.model.edit(original);
    await f.wrapper.get('input').setValue('Welcome message');
    await f.wrapper.findAll('input')[1]!.setValue('Welcome, {{ event.user.uniqueId }}');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('The open text will be replaced');
    expect(f.wrapper.get('dialog[open]').text()).toContain('Welcome message');
    await f.dialogButton('Cancel').trigger('click');
    expect(f.model.text.value).toBe(original);
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Welcome message', enabled: false,
      typeId: 'core.log', config: { message: 'Welcome, {{ event.user.uniqueId }}' }, schemaVersion: 2 });
    expect(f.model.saved.value).toBeNull();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('generates a disabled chat event with a bounded filter and one selected action', async () => {
  const f = await setup('event');
  try {
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Keyword alert');
    await inputs[1]!.setValue('hello');
    await inputs[2]!.setValue('author.action');
    await inputs[3]!.setValue('1500');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Keyword alert', enabled: false,
      trigger: 'tiktok.chat', actionIds: ['author.action'], filters: [{ path: 'event.data.comment', operator: 'contains', value: 'hello' }],
      cooldownMs: 1500, cooldownScope: 'global', runMode: 'all', schemaVersion: 1 });
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('invalidates an open confirmation when the text or authority changes', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.get('input').setValue('Test');
    await f.wrapper.findAll('input')[1]!.setValue('message');
    await f.button('Prepare draft').trigger('click');
    f.model.edit('newer unsaved text');
    await flushPromises();
    expect(f.dialogButton('Use this draft').attributes('disabled')).toBeDefined();
    await f.dialogButton('Use this draft').trigger('click');
    expect(f.model.text.value).toBe('newer unsaved text');
    f.status.value.hello!.grantRevision = '2';
    await flushPromises();
    expect(f.wrapper.find('dialog[open]').exists()).toBe(false);
    expect(f.model.text.value).toBe('');
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('rejects invalid waits and action identifiers without changing the source', async () => {
  const f = await setup('event');
  try {
    const original = f.model.text.value;
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Example');
    await inputs[2]!.setValue('bad*id');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await inputs[2]!.setValue('valid.id');
    await inputs[3]!.setValue('86400001');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    expect(f.model.text.value).toBe(original);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});
