import { setup, t, expect, it, flushPromises } from './testing/proposal-builder-fixture.ts';

it('loads a shared action into fields and preserves unrelated properties', async () => {
  const f = await setup('action');
  try {
    f.model.edit(JSON.stringify({ id: 'author.example', name: 'Existing', enabled: false, typeId: 'core.log', schemaVersion: 2, config: { message: 'before', custom: 'keep' }, custom: { keep: true } }));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect((f.wrapper.findAll('input')[0]!.element as HTMLInputElement).value).toBe('Existing');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('before');
    await f.wrapper.findAll('input')[1]!.setValue('after');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toEqual({ id: 'author.example', name: 'Existing', enabled: false, typeId: 'core.log', schemaVersion: 2, config: { message: 'after', custom: 'keep' }, custom: { keep: true } });
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('does not carry configuration from the previous action type', async () => {
  const f = await setup('action');
  try {
    f.model.edit(JSON.stringify({ id:'author.example', name:'Existing', enabled:false, typeId:'core.log', schemaVersion:2, config:{ message:'before', privateOption:'old' }, custom:'keep' }));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    await f.wrapper.findAll('select')[0]!.setValue('core.delay');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ typeId:'core.emit', enabled:false, config:{type:'overlay.alert', data:{user:'{{event.user.uniqueId}}'}} });
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('overlay.alert');
    await f.wrapper.findAll('input')[1]!.setValue('INVALID TOPIC');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[1]!.setValue('overlay.alert');
    await f.wrapper.get('textarea').setValue('[1,2]');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.get('textarea').setValue('{"n":1e400}');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.get('textarea').setValue('{"deep":' + '['.repeat(40) + '0' + ']'.repeat(40) + '}');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('prepares and reloads a bounded HTTP action without issuing a request', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Webhook');
    await f.wrapper.findAll('select')[0]!.setValue('core.fetch');
    const inputs = f.wrapper.findAll('input');
    await inputs[1]!.setValue('https://hooks.example/live');
    await inputs[2]!.setValue('3000');
    await inputs[3]!.setValue('1024');
    const textareas = f.wrapper.findAll('textarea');
    await textareas[0]!.setValue('{"content-type":"application/json"}');
    await textareas[1]!.setValue('{"comment":"{{event.data.comment}}"}');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({typeId:'core.fetch', enabled:false, config:{url:'https://hooks.example/live', method:'POST', headers:{'content-type':'application/json'}, body:'{"comment":"{{event.data.comment}}"}', timeoutMs:3000, maxResponseBytes:1024, allowPrivateNetwork:false}});
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('https://hooks.example/live');
    await f.wrapper.findAll('input')[1]!.setValue('http://127.0.0.1/hook');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[4]!.setValue(true);
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeUndefined();
    await f.wrapper.findAll('textarea')[0]!.setValue('{"X-Bad":42}');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('prepares host audio with explicit volume and overlap without playing it', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Alert sound');
    await f.wrapper.findAll('select')[0]!.setValue('audio.play');
    expect(f.wrapper.text()).toContain('path is on the host PC');
    await f.wrapper.findAll('input')[1]!.setValue('/media/alert.wav');
    await f.wrapper.findAll('input')[2]!.setValue('0.5');
    await f.wrapper.findAll('select')[1]!.setValue('drop');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({typeId:'audio.play',enabled:false,config:{fileRef:'/media/alert.wav',volume:0.5,overlap:'drop'}});
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    await f.wrapper.findAll('input')[1]!.setValue('{{globals.secret}}');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[1]!.setValue('/media/alert.wav');
    await f.wrapper.findAll('input')[2]!.setValue('2');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('prepares and reloads bounded code without running it on the controller', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Script');
    await f.wrapper.findAll('select')[0]!.setValue('core.code');
    expect(f.wrapper.text()).toContain('no direct file, network or private-global access');
    const code = 'return {log:[event.user.uniqueId]};';
    await f.wrapper.get('textarea').setValue(code);
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({typeId:'core.code',enabled:false,config:{source:code}});
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect((f.wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe(code);
    await f.wrapper.get('textarea').setValue('é'.repeat(65_537));
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
    expect(f.calls.map((call) => call.method)).toEqual(['controller.proposals.list']);
    f.status.value.hello!.grantRevision = '2';
    await flushPromises();
    expect(f.wrapper.find('textarea').exists()).toBe(false);
    expect(f.model.text.value).toBe('');
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('replaces stale file metadata only when the host audio path changes', async () => {
  const f = await setup('action');
  try {
    f.model.edit(JSON.stringify({id:'author.example',name:'Sound',enabled:false,typeId:'core.audio.play',schemaVersion:2,config:{fileRef:{path:'/media/old.wav',name:'old.wav',sizeBytes:12},volume:0.5,overlap:'allow',custom:'keep'}}));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click'); await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value).config.fileRef).toEqual({path:'/media/old.wav',name:'old.wav',sizeBytes:12});
    await f.wrapper.findAll('input')[1]!.setValue('/media/new.wav');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click'); await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({typeId:'core.audio.play',config:{fileRef:{path:'/media/new.wav'},custom:'keep'}});
    expect(JSON.parse(f.model.text.value).config.fileRef).toEqual({path:'/media/new.wav'});
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('preserves extra HTTP settings and refuses unsupported response emission', async () => {
  const f = await setup('action');
  try {
    const record = {id:'author.example', name:'Webhook', enabled:false, typeId:'core.fetch', schemaVersion:2, custom:'keep', config:{url:'https://hooks.example/live',method:'GET',headers:{},body:null,timeoutMs:5000,maxResponseBytes:128,allowPrivateNetwork:false,custom:'keep'}};
    f.model.edit(JSON.stringify(record));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    await f.wrapper.findAll('input')[2]!.setValue('6000');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({custom:'keep',config:{custom:'keep',timeoutMs:6000}});
    const unsupported = JSON.stringify({...record, config:{...record.config,emitResponseAs:'private.response'}});
    f.model.edit(unsupported);
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect(f.wrapper.text()).toContain('These fields cannot be loaded');
    expect(f.model.text.value).toBe(unsupported);
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ trigger:'overlay.alert', actionIds:['action.one'], filters:[], enabled:false });
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect((f.wrapper.findAll('input')[1]!.element as HTMLInputElement).value).toBe('overlay.alert');
    await f.wrapper.findAll('input')[1]!.setValue('tiktok.unknown');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await f.wrapper.findAll('input')[1]!.setValue('tiktok.chat');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('loads an existing event without dropping unrelated fields or ordered actions', async () => {
  const f = await setup('event');
  try {
    f.model.edit(JSON.stringify({ id:'author.example', name:'Existing event', enabled:false, trigger:'tiktok.gift', actionIds:['one', 'two'], filters:[{path:'event.data.giftName',operator:'contains',value:'Rose'}], cooldownMs:1000, cooldownScope:'user', runMode:'random', schemaVersion:1, custom:{keep:true} }));
    await f.button(t('controller.proposalBuilder.labels.loadFields')).trigger('click');
    expect(f.wrapper.text()).toContain('one');
    expect(f.wrapper.text()).toContain('two');
    await f.wrapper.findAll('input')[1]!.setValue('Flower');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(JSON.parse(f.model.text.value)).toMatchObject({ name:'Existing event', actionIds:['one', 'two'], filters:[{path:'event.data.giftName',operator:'contains',value:'Flower'}], cooldownMs:1000, cooldownScope:'user', runMode:'random', custom:{keep:true} });
  } finally { f.wrapper.unmount(); f.model.stop(); }
});
