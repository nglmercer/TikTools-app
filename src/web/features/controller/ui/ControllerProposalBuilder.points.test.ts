
import { setup, t, expect, it, flushPromises, ref, createAutomationModel, type AutomationRow } from './testing/proposal-builder-fixture.ts';

it('clears added actions and closes the preview when the controller grant changes', async () => {
  const f = await setup('event');
  try {
    await f.wrapper.findAll('input')[0]!.setValue('Scoped event');
    await f.wrapper.findAll('input')[2]!.setValue('private.action');
    await f.button(t('controller.proposalBuilder.labels.addAction')).trigger('click');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    // User-typed fixture text, not a translated UI label.
    await inputs[0]!.setValue('Adjust points');
    await inputs[1]!.setValue('alice');
    await inputs[2]!.setValue('-12.5');
    expect(f.wrapper.text()).toContain('this draft does not adjust points');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('"uniqueId": "alice"');
    expect(f.model.text.value).toBe('existing private draft');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    expect(f.wrapper.get('dialog[open]').text()).toContain('{{event.user.uniqueId}}');
    expect(f.model.text.value).toBe('existing private draft');
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await f.wrapper.findAll('select')[1]!.setValue('specific');
    expect(f.dialogButton(t('controller.proposalBuilder.labels.apply')).attributes('disabled')).toBeDefined();
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
      expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    }
    await inputs[1]!.setValue('alice');
    for (const value of ['0', '1000000001', '-1000000001']) {
      await inputs[2]!.setValue(value);
      expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
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
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    await inputs[1]!.setValue('bob');
    expect(f.dialogButton(t('controller.proposalBuilder.labels.apply')).attributes('disabled')).toBeDefined();
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
    expect(f.model.text.value).toBe(original);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});

it('invalidates an open confirmation when the text or authority changes', async () => {
  const f = await setup('action');
  try {
    await f.wrapper.get('input').setValue('Test');
    await f.wrapper.findAll('input')[1]!.setValue('message');
    await f.button(t('controller.proposalBuilder.labels.generate')).trigger('click');
    f.model.edit('newer unsaved text');
    await flushPromises();
    expect(f.dialogButton(t('controller.proposalBuilder.labels.apply')).attributes('disabled')).toBeDefined();
    await f.dialogButton(t('controller.proposalBuilder.labels.apply')).trigger('click');
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
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    await inputs[2]!.setValue('valid.id');
    await inputs[3]!.setValue('86400001');
    expect(f.button(t('controller.proposalBuilder.labels.generate')).attributes('disabled')).toBeDefined();
    expect(f.model.text.value).toBe(original);
    expect(f.send).not.toHaveBeenCalled();
  } finally { f.wrapper.unmount(); f.model.stop(); }
});
