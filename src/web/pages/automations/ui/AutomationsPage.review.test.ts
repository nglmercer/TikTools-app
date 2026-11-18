import { chatEvent, giftEvent, t, mountPage, buttonByText, goBack, documentButton, selectRow, it, expect, nextTick, ControlCallError, flushPromises, type LiveEvent, afterEach, beforeEach, resetResolutionPreference } from './automations-page-fixture.ts';

beforeEach(() => { localStorage.clear(); resetResolutionPreference('manual'); });
afterEach(() => resetResolutionPreference('automatic'));


it('pins the base across background refreshes, preserves a conflicting draft and rebases without resubmitting', async () => {
    const { wrapper, snapshot, versions, automation, requests, calls } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('My draft');
    snapshot.events[0] = { ...giftEvent, name: 'Edited on host', enabled: false };
    versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('My draft');
    expect(wrapper.text()).toContain('changed since you opened it');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(requests.find((call) => call.method === 'automation.update')?.meta).toEqual({ expectedRevision: '1' });
    expect(wrapper.text()).toContain('Your draft is retained');
    expect((buttonByText(wrapper, t('automations.editor.save')).element as HTMLButtonElement).disabled).toBe(true);
    await buttonByText(wrapper, t('automations.conflict.compare')).trigger('click'); await flushPromises();
    expect(document.body.textContent).toContain(t('automations.conflict.base'));
    const sources = Array.from(document.querySelectorAll('.base-dialog .review-table .review-table__action')) as HTMLElement[];
    expect(sources).toHaveLength(5);
    sources[2]!.click(); await flushPromises();
    expect(document.body.textContent).toContain('Edited on host');
    sources[1]!.click(); await flushPromises();
    expect(document.body.textContent).toContain('My draft');
    const writes = calls.filter((call) => call.method === 'automation.update').length;
    documentButton(t('automations.conflict.useBase')).click(); await flushPromises();
    expect(calls.filter((call) => call.method === 'automation.update')).toHaveLength(writes);
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('My draft');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(requests.filter((call) => call.method === 'automation.update').at(-1)?.meta).toEqual({ expectedRevision: '2' });
    expect(versions.get('event:e1')).toBe('3');
    expect(wrapper.text()).not.toContain(t('automations.editor.unsaved'));
  });

  it('does not reset even an untouched opening base when the host changes it', async () => {
    const { wrapper, snapshot, versions, automation, requests } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    snapshot.events[0] = { ...giftEvent, name: 'New host name' }; versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Edited from old base');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.update')?.meta?.expectedRevision).toBe('1');
  });

  it('keeps a deleted record draft and never falls back from update to create', async () => {
    const { wrapper, snapshot, versions, automation, calls, requests } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Keep this text');
    snapshot.events.splice(0, 1); versions.set('event:e1', '2');
    await automation.refresh(); await flushPromises();
    expect(wrapper.find('.event-editor').exists()).toBe(true);
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(calls.some((r) => r.method === 'automation.create')).toBe(false);
    await buttonByText(wrapper, t('automations.conflict.compare')).trigger('click'); await flushPromises();
    expect(document.body.textContent).toContain('This ID is absent');
    documentButton(t('automations.conflict.prepareRecreate')).click(); await flushPromises();
    expect(calls.some((r) => r.method === 'automation.create')).toBe(false);
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.create')?.meta?.expectedRevision).toBe('2');
    expect(snapshot.events.some((r) => r.id === 'e1' && r.name === 'Keep this text')).toBe(true);
  });

  it('retains dirty text on a failed save and asks before navigation', async () => {
    const { wrapper, calls } = await mountPage((method) => {
      if (method === 'automation.update') return Promise.reject(new ControlCallError('timeout', 'Acknowledgement lost'));
      return undefined;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Unconfirmed draft');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('outcome is unconfirmed');
    expect(wrapper.text()).toContain(t('automations.editor.unsaved'));
    expect(calls.filter((r) => r.method === 'automation.definition.submit' && r.params?.operation === 'update')).toHaveLength(1);
    await goBack(wrapper);
    expect(document.body.textContent).toContain(t('automations.editor.discardTitle'));
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Unconfirmed draft');
  });

  it('preserves newer form edits when a delayed save is acknowledged', async () => {
    let finish!: (record: LiveEvent) => void;
    let submitted!: LiveEvent;
    const pending = new Promise<LiveEvent>((resolve) => { finish = resolve; });
    const { wrapper, snapshot, versions } = await mountPage((method, params, options) => {
      if (method !== 'automation.update') return undefined;
      submitted = params?.record as LiveEvent;
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'event', id: 'e1', revision: '2' }] });
      return pending;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Submitted text');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click');
    await wrapper.get('.event-editor input[type="text"]').setValue('Newer text');
    snapshot.events[0] = submitted; versions.set('event:e1', '2'); finish(submitted);
    await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Newer text');
    expect(wrapper.text()).toContain(t('automations.editor.unsaved'));
    expect((buttonByText(wrapper, t('automations.editor.save')).element as HTMLButtonElement).disabled).toBe(false);
  });

  it('a delayed acknowledgement cannot select or clear another editor after explicit navigation', async () => {
    let finish!: (record: LiveEvent) => void;
    let submitted!: LiveEvent;
    const pending = new Promise<LiveEvent>((resolve) => { finish = resolve; });
    const { wrapper, snapshot, versions } = await mountPage((method, params, options) => {
      if (method !== 'automation.update') return undefined;
      submitted = params?.record as LiveEvent;
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'event', id: 'e1', revision: '2' }] });
      return pending;
    });
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Pending first edit');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click');
    await goBack(wrapper); documentButton(t('automations.editor.discardConfirm')).click(); await flushPromises();
    await selectRow(wrapper, 'Chat hello');
    await wrapper.get('.event-editor input[type="text"]').setValue('Second draft');
    snapshot.events[0] = submitted; versions.set('event:e1', '2'); finish(submitted); await flushPromises();
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Second draft');
    expect(wrapper.text()).toContain(t('automations.editor.unsaved'));
  });

  it('failed deletion keeps the editor and its unsaved candidate', async () => {
    const { wrapper } = await mountPage((method) => method === 'automation.delete'
      ? Promise.reject(new ControlCallError('revision_conflict', 'Changed before delete')) : undefined);
    await selectRow(wrapper, 'Chat hello');
    await wrapper.get('.event-editor input[type="text"]').setValue('Keep after delete failed');
    await buttonByText(wrapper, t('automations.editor.delete')).trigger('click'); await nextTick(); documentButton(t('automations.editor.deleteConfirm')).click(); await flushPromises();
    expect(wrapper.find('.event-editor').exists()).toBe(true);
    expect((wrapper.get('.event-editor input[type="text"]').element as HTMLInputElement).value).toBe('Keep after delete failed');
    expect(wrapper.text()).toContain('revision_conflict');
  });

  it('canceling enablement restores the committed checkbox without a write', async () => {
    const { wrapper, calls } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    await nextTick(); documentButton(t('common.cancel')).click(); await flushPromises();
    expect((wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').element as HTMLInputElement).checked).toBe(true);
    expect(calls.some((r) => r.method === 'automation.disable')).toBe(false);
  });

  it('an enablement conflict requires a fresh read and another explicit confirmation', async () => {
    const { wrapper, snapshot, versions, requests } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    snapshot.events[1] = { ...chatEvent, name: 'New name before confirmation' }; versions.set('event:e2', '2');
    documentButton(t('automations.conflict.confirmToggle')).click(); await flushPromises();
    expect(requests.find((r) => r.method === 'automation.disable')?.meta?.expectedRevision).toBe('1');
    expect(document.body.textContent).toContain('revision_conflict');
    documentButton(t('automations.conflict.readCurrent')).click(); await flushPromises();
    expect(document.body.textContent).toContain('New name before confirmation');
    expect(requests.filter((r) => r.method === 'automation.disable')).toHaveLength(1);
    documentButton(t('automations.conflict.confirmToggle')).click(); await flushPromises();
    expect(requests.filter((r) => r.method === 'automation.disable').at(-1)?.meta?.expectedRevision).toBe('2');
  });
