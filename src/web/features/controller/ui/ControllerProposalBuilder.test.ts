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

it('loads a shared action into fields and preserves unrelated properties', async () => {
  const f = await setup('action');
  try {
    f.model.edit(JSON.stringify({ id: 'author.example', name: 'Existing', enabled: false, typeId: 'core.log', schemaVersion: 2, config: { message: 'before', custom: 'keep' }, custom: { keep: true } }));
    await f.button('Load fields from open text').trigger('click');
    expect((f.wrapper.findAll('input')[0]!.element as HTMLInputElement).value).toBe('Existing');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('before');
    await f.wrapper.findAll('input')[1]!.setValue('after');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Existing', enabled: false, typeId: 'core.log', schemaVersion: 2, config: { message: 'after', custom: 'keep' }, custom: { keep: true } });
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('does not carry configuration from the previous action type', async () => {
  const f = await setup('action');
  try {
    f.model.edit(JSON.stringify({ id:'author.example', name:'Existing', enabled:false, typeId:'core.log', schemaVersion:2, config:{ message:'before', privateOption:'old' }, custom:'keep' }));
    await f.button('Load fields from open text').trigger('click');
    await f.wrapper.findAll('select')[0]!.setValue('core.delay');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ typeId:'core.delay', config:{ms:1000}, custom:'keep' });
    expect(JSON.parse(f.model.text.value).config).toEqual({ms:1000});
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('builds and reloads a bounded internal-event action without sending it', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Notify overlay');
    await f.wrapper.findAll('select')[0]!.setValue('core.emit');
    await f.wrapper.findAll('input')[1]!.setValue('overlay.alert');
    await f.wrapper.get('textarea').setValue('{"user":"{{event.user.uniqueId}}"}');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ typeId:'core.emit', enabled:false, config:{type:'overlay.alert', data:{user:'{{event.user.uniqueId}}'}} });
    await f.button('Load fields from open text').trigger('click');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('overlay.alert');
    await f.wrapper.findAll('input')[1]!.setValue('INVALID TOPIC');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[1]!.setValue('overlay.alert');
    await f.wrapper.get('textarea').setValue('[1,2]');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await f.wrapper.get('textarea').setValue('{"n":1e400}');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await f.wrapper.get('textarea').setValue('{"deep":' + '['.repeat(40) + '0' + ']'.repeat(40) + '}');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('builds and reloads an internal-event trigger with a literal topic', async () => {
  const f = await setup('event');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Overlay handler');
    await f.wrapper.findAll('select')[0]!.setValue('custom');
    expect(f.wrapper.text()).toContain('only receives authorized internal events');
    await f.wrapper.findAll('input')[1]!.setValue('overlay.alert');
    await f.wrapper.findAll('input')[2]!.setValue('action.one');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ trigger:'overlay.alert', actionIds:['action.one'], filters:[], enabled:false });
    await f.button('Load fields from open text').trigger('click');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('overlay.alert');
    await f.wrapper.findAll('input')[1]!.setValue('tiktok.unknown');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[1]!.setValue('tiktok.chat');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('loads an existing event without dropping unrelated fields or ordered actions', async () => {
  const f = await setup('event');
  try {
    f.model.edit(JSON.stringify({ id:'author.example', name:'Existing event', enabled:false, trigger:'tiktok.gift', actionIds:['one', 'two'], filters:[{path:'event.data.giftName',operator:'contains',value:'Rose'}], cooldownMs:1000, cooldownScope:'user', runMode:'random', schemaVersion:1, custom:{keep:true} }));
    await f.button('Load fields from open text').trigger('click');
    expect(f.wrapper.text()).toContain('one');
    expect(f.wrapper.text()).toContain('two');
    await f.wrapper.findAll('input')[1]!.setValue('Flower');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ name:'Existing event', actionIds:['one', 'two'], filters:[{path:'event.data.giftName',operator:'contains',value:'Flower'}], cooldownMs:1000, cooldownScope:'user', runMode:'random', custom:{keep:true} });
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('rejects unsupported filters and stale imported fields without changing text', async () => {
  const f = await setup('event');
  try {
    const unsupported = JSON.stringify({ id:'author.example', name:'Complex', enabled:false, trigger:'tiktok.chat', actionIds:[], filters:[{path:'event.data.comment',operator:'eq',value:'hello'}], schemaVersion:1 });
    f.model.edit(unsupported);
    await f.button('Load fields from open text').trigger('click');
    expect(f.wrapper.text()).toContain('These fields cannot be loaded');
    expect(f.model.text.value).toBe(unsupported);
    f.model.edit(JSON.stringify({ id:'author.example', name:'Simple', enabled:false, trigger:'tiktok.chat', actionIds:[], filters:[], schemaVersion:1 }));
    await f.button('Load fields from open text').trigger('click');
    f.model.edit(f.model.text.value + ' ');
    await flushPromises();
    expect(f.wrapper.text()).toContain('The open text changed');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

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

it('prepares a gift-name condition and a per-viewer cooldown for owner review', async () => {
  const f = await setup('event');
  try {
    const original = f.model.text.value;
    await f.wrapper.findAll('select')[0]!.setValue('tiktok.gift');
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Reward roses');
    await inputs[1]!.setValue('Rose');
    await inputs[2]!.setValue('points.action');
    await inputs[3]!.setValue('1000');
    await f.wrapper.findAll('select')[2]!.setValue('user');
    expect(f.wrapper.text()).toContain('Events without a viewer ID share a delay');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"cooldownScope": "user"');
    expect(f.model.text.value).toBe(original);
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id:'author.example', name:'Reward roses', enabled:false,
      trigger:'tiktok.gift', actionIds:['points.action'], filters:[{ path:'event.data.giftName', operator:'contains', value:'Rose' }],
      cooldownMs:1000, cooldownScope:'user', runMode:'all', schemaVersion:1 });
    expect(f.model.saved.value).toBeNull();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('does not carry a hidden chat filter into a gift event and invalidates changed cooldown review', async () => {
  const f = await setup('event');
  try {
    const original = f.model.text.value;
    await f.wrapper.findAll('input')[0]!.setValue('Gift event');
    await f.wrapper.findAll('input')[1]!.setValue('private chat text');
    await f.wrapper.findAll('select')[0]!.setValue('tiktok.gift');
    await f.wrapper.findAll('input')[3]!.setValue('1000');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"filters": []');
    expect(f.wrapper.get('dialog[open]').text()).not.toContain('private chat text');
    await f.wrapper.findAll('select')[2]!.setValue('user');
    expect(f.dialogButton('Use this draft').attributes('disabled')).toBeDefined();
    await f.dialogButton('Use this draft').trigger('click');
    expect(f.model.text.value).toBe(original);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('rejects a gift condition exceeding the UTF-8 byte bound', async () => {
  const f = await setup('event');
  try {
    await f.wrapper.findAll('select')[0]!.setValue('tiktok.gift');
    await f.wrapper.findAll('input')[0]!.setValue('Event');
    await f.wrapper.findAll('input')[1]!.setValue('é'.repeat(251));
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('prepares several ordered actions and invalidates a preview when their order changes', async () => {
  const f = await setup('event');
  try {
    const original = f.model.text.value;
    await f.wrapper.findAll('input')[0]!.setValue('Welcome and reward');
    const actionInput = () => f.wrapper.findAll('input')[2]!;
    await actionInput().setValue('log.action');
    await f.button('Add action').trigger('click');
    await actionInput().setValue('points.action');
    await f.button('Add action').trigger('click');
    await actionInput().setValue('audio.action');
    await f.wrapper.findAll('select')[0]!.setValue('tiktok.chat');
    await f.wrapper.findAll('select')[1]!.setValue('random');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"runMode": "random"');
    expect(f.model.text.value).toBe(original);
    await f.wrapper.get('button[aria-label="Move points.action up"]').trigger('click');
    expect(f.dialogButton('Use this draft').attributes('disabled')).toBeDefined();
    await f.dialogButton('Use this draft').trigger('click');
    expect(f.model.text.value).toBe(original);
    await f.dialogButton('Cancel').trigger('click');
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ enabled:false, runMode:'random',
      actionIds:['points.action', 'log.action', 'audio.action'] });
    expect(f.model.saved.value).toBeNull();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('rejects duplicate action IDs until the moderator removes or replaces one', async () => {
  const f = await setup('event');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Example');
    const actionInput = () => f.wrapper.findAll('input')[2]!;
    await actionInput().setValue('same.action');
    await f.button('Add action').trigger('click');
    await actionInput().setValue('same.action');
    expect(f.wrapper.text()).toContain('Use valid, unique IDs');
    expect(f.button('Prepare draft').attributes('disabled')).toBeDefined();
    await f.wrapper.get('button[aria-label="Remove same.action"]').trigger('click');
    expect(f.button('Prepare draft').attributes('disabled')).toBeUndefined();
    await f.button('Prepare draft').trigger('click');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value).actionIds).toEqual(['same.action']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('clears added actions and closes the preview when the controller grant changes', async () => {
  const f = await setup('event');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Scoped event');
    await f.wrapper.findAll('input')[2]!.setValue('private.action');
    await f.button('Add action').trigger('click');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('private.action');
    f.status.value.hello!.grantRevision = '2';
    await flushPromises();
    expect(f.wrapper.find('dialog[open]').exists()).toBe(false);
    expect(f.wrapper.text()).not.toContain('private.action');
    expect(f.model.text.value).toBe('');
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

it('prepares points for the viewer in each triggering event, without sending or activating them', async () => {
  const f = await setup('action');
  try {
    f.model.edit('existing private draft');
    await f.wrapper.get('select').setValue('core.points');
    await f.wrapper.findAll('select')[1]!.setValue('event');
    const inputs = f.wrapper.findAll('input');
    await inputs[0]!.setValue('Reward the participant');
    await inputs[1]!.setValue('5');
    expect(f.wrapper.text()).toContain('permissions are checked on every execution');
    expect(f.wrapper.text()).toContain('this draft does not adjust points');
    await f.button('Prepare draft').trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('{{event.user.uniqueId}}');
    expect(f.model.text.value).toBe('existing private draft');
    await f.dialogButton('Use this draft').trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Reward the participant', enabled: false,
      typeId: 'core.points', config: { uniqueId: '{{event.user.uniqueId}}', delta: 5 }, schemaVersion: 2 });
    expect(f.model.saved.value).toBeNull();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('invalidates a points preview when changing from the triggering viewer to a specific one', async () => {
  const f = await setup('action');
  try {
    const original = f.model.text.value;
    await f.wrapper.get('select').setValue('core.points');
    await f.wrapper.findAll('select')[1]!.setValue('event');
    await f.wrapper.findAll('input')[0]!.setValue('Reward');
    await f.wrapper.findAll('input')[1]!.setValue('5');
    await f.button('Prepare draft').trigger('click');
    await f.wrapper.findAll('select')[1]!.setValue('specific');
    expect(f.dialogButton('Use this draft').attributes('disabled')).toBeDefined();
    await f.dialogButton('Use this draft').trigger('click');
    expect(f.model.text.value).toBe(original);
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
