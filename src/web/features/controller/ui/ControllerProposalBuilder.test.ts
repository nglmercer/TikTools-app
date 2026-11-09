import { mount, flushPromises } from '@vue/test-utils';
import { expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createProposalModel } from '../model/proposals.ts';
import { createAutomationModel, type AutomationRow } from '../model/automations.ts';
import type { ControllerStatus } from '../model/controller.ts';
import ControllerProposalBuilder from './ControllerProposalBuilder.vue';

async function setup(kind: 'action' | 'event') {
  await activateLocale('en');
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } } });
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.proposals.list') return { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', storeRevision: '0', rows: [], next: null };
    if (method === 'controller.status') return JSON.parse(JSON.stringify(status.value));
    if (method === 'controller.automations') return { epoch: '1', automationRevision: '1', rows: [
      { summary: { kind: 'action', id: 'alert.action', name: 'Alert <unsafe>', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: 'action', id: 'alert.action', revision: '2' } },
      { summary: { kind: 'event', id: 'other.event', name: 'Other event', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: 'event', id: 'other.event', revision: '1' } },
    ], next: null };
    throw new Error(`unexpected ${method}`);
  });
  const send = vi.fn(async () => true);
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send });
  await model.refresh(); model.select(kind, 'author.example');
  const wrapper = mount(ControllerProposalBuilder, { props: { model }, global: { plugins: [i18n] } });
  const button = (label: string) => wrapper.findAll('button').find((node) => node.text() === label)!;
  const dialogButton = (label: string) => wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === label)!;
  return { status, model, wrapper, button, dialogButton, calls, send, client };
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

it('selects a permitted host action without saving or sending the proposal', async () => {
  const f = await setup('event');
  f.status.value.automationRevision = '1';
  f.status.value.hello!.features['automationSummary'] = true;
  const automation = createAutomationModel(f.client, { status: f.status, generation: () => 1, stopped: () => false,
    retry: () => {}, blocked: () => false, journalMatches: () => true, send: f.send });
  try {
    await automation.refresh(f.status.value, 1);
    await f.wrapper.setProps({ automation });
    const section = f.wrapper.get('section.available-actions');
    expect(section.text()).toContain('Alert <unsafe>');
    expect(section.find('img').exists()).toBe(false);
    expect(section.text()).not.toContain('other.event');
    await section.get('input').setValue('ALERT');
    await section.get('button').trigger('click');
    expect((f.wrapper.findAll('input')[2]!.element as HTMLInputElement).value).toBe('alert.action');
    expect(f.model.text.value).not.toContain('alert.action');
    await f.wrapper.get('input').setValue('Alert on chat');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value).actionIds).toEqual(['alert.action']);
    expect(f.model.saved.value).toBeNull();
    expect(f.send).not.toHaveBeenCalled();
    f.status.value.hello!.capabilities = [];
    await flushPromises();
    expect(f.wrapper.find('section.available-actions').exists()).toBe(false);
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('bounds the displayed matches and lets the moderator search by ID', async () => {
  const f = await setup('event');
  const rows: AutomationRow[] = Array.from({ length: 51 }, (_, index) => ({
    summary: { kind: 'action', id: `action.${index}`, name: `Action ${index}`, enabled: false, definitionRedacted: true },
    version: { kind: 'automation', scope: 'action', id: `action.${index}`, revision: '1' },
  }));
  const automation = { ready: ref(true), rows: ref(rows) } as unknown as ReturnType<typeof createAutomationModel>;
  try {
    await f.wrapper.setProps({ automation });
    const section = f.wrapper.get('section.available-actions');
    expect(section.findAll('li')).toHaveLength(50);
    expect(section.text()).toContain('Showing the first 50 matches');
    await section.get('input').setValue('action.50');
    expect(section.findAll('li')).toHaveLength(1);
    await section.get('button').trigger('click');
    expect((f.wrapper.findAll('input')[2]!.element as HTMLInputElement).value).toBe('action.50');
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('prepares a literal-viewer points proposal only after explicit preview', async () => {
  const f = await setup('action');
  try {
    f.model.edit('existing private draft');
    await f.wrapper.get('select').setValue('core.points');
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Adjust points');
    await inputs[1]!.setValue('alice');
    await inputs[2]!.setValue('-12.5');
    expect(f.wrapper.text()).toContain('this draft does not adjust points');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"uniqueId": "alice"');
    expect(f.model.text.value).toBe('existing private draft');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Adjust points', enabled: false,
      typeId: 'core.points', config: { uniqueId: 'alice', delta: -12.5 }, schemaVersion: 2 });
    expect(f.model.saved.value).toBeNull();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('rejects dynamic, invalid, and excessive points targets without changing the draft', async () => {
  const f = await setup('action');
  try {
    const original = f.model.text.value;
    await f.wrapper.get('select').setValue('core.points');
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Points');
    await inputs[2]!.setValue('1');
    for (const id of ['@alice', '{{ event.user.uniqueId }}', 'bad*id', ' alice']) {
      await inputs[1]!.setValue(id);
      expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    }
    await inputs[1]!.setValue('alice');
    for (const value of ['0', '1000000001', '-1000000001']) {
      await inputs[2]!.setValue(value);
      expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    }
    expect(f.model.text.value).toBe(original);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('invalidates a points preview if the target or amount changes', async () => {
  const f = await setup('action');
  try {
    const original = f.model.text.value;
    await f.wrapper.get('select').setValue('core.points');
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Points');
    await inputs[1]!.setValue('alice');
    await inputs[2]!.setValue('5');
    await f.button('Prepare draft').trigger('click');
    await inputs[1]!.setValue('bob');
    expect(f.dialogButton('Use this draft').attributes('disabled')).toBeDefined();
    await f.dialogButton('Use this draft').trigger('click');
    expect(f.model.text.value).toBe(original);
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
