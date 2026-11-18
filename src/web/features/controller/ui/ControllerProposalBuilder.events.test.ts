
import { setup, t, expect, it, flushPromises } from './testing/proposal-builder-fixture.ts';

it('rejects unsupported filters and stale imported fields without changing text', async () => {
  const f = await setup('event');
  try {
    const unsupported = JSON.stringify({ id:'author.example', name:'Complex', enabled:false, trigger:'tiktok.chat', actionIds:[], filters:[{path:'event.data.comment',operator:'eq',value:'hello'}], schemaVersion:1 });
    f.model.edit(unsupported);
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect(f.wrapper.text()).toContain('These fields cannot be loaded');
    expect(f.model.text.value).toBe(unsupported);
    f.model.edit(JSON.stringify({ id:'author.example', name:'Simple', enabled:false, trigger:'tiktok.chat', actionIds:[], filters:[], schemaVersion:1 }));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    f.model.edit(f.model.text.value + ' ');
    await flushPromises();
    expect(f.wrapper.text()).toContain('The open text changed');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('The open text will be replaced');
    expect(f.wrapper.get('dialog[open]').text()).toContain('Welcome message');
    await f.dialogButton(t('common.cancel')).trigger('click');
    expect(f.model.text.value).toBe(original);
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"cooldownScope": "user"');
    expect(f.model.text.value).toBe(original);
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"filters": []');
    expect(f.wrapper.get('dialog[open]').text()).not.toContain('private chat text');
    await f.wrapper.findAll('select')[2]!.setValue('user');
    expect(f.dialogButton(t('controller.proposalBuilder.labels.apply')).attributes('disabled')).toBeDefined();
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
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
    await f.button(t('controller.proposalBuilder.labels.addAction')).trigger('click');
    await actionInput().setValue('points.action');
    await f.button(t('controller.proposalBuilder.labels.addAction')).trigger('click');
    await actionInput().setValue('audio.action');
    await f.wrapper.findAll('select')[0]!.setValue('tiktok.chat');
    await f.wrapper.findAll('select')[1]!.setValue('random');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"runMode": "random"');
    expect(f.model.text.value).toBe(original);
    await f.wrapper.get('button[aria-label="Move points.action up"]').trigger('click');
    expect(f.dialogButton(t('controller.proposalBuilder.labels.apply')).attributes('disabled')).toBeDefined();
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(f.model.text.value).toBe(original);
    await f.dialogButton(t('common.cancel')).trigger('click');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.addAction')).trigger('click');
    await actionInput().setValue('same.action');
    expect(f.wrapper.text()).toContain('Use valid, unique IDs');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.get('button[aria-label="Remove same.action"]').trigger('click');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeUndefined();
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value).actionIds).toEqual(['same.action']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});
