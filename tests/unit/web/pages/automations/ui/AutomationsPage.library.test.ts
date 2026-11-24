import { t, mountPage, buttonByLabel, goBack, documentButton, selectRow, it, expect, nextTick, libraryJournal, flushPromises, afterEach, beforeEach, resetResolutionPreference } from '../../../../../../src/web/pages/automations/ui/automations-page-fixture.ts';

beforeEach(() => { localStorage.clear(); resetResolutionPreference('manual'); });
afterEach(() => resetResolutionPreference('automatic'));


it('blocks profile and template changes until the initial coherent library is read and permits a read-only retry after failure', async () => {
    let reject!: (failure: unknown) => void;
    const held = new Promise<never>((_resolve, fail) => { reject = fail; });
    let blocked = true;
    const f = await mountPage((method) => method === 'automation.snapshot' && blocked ? held : undefined, false);
    const profile = f.wrapper.get('.profile-toolbar [role="combobox"]');
    const menu = f.wrapper.get('.profile-toolbar [aria-haspopup="menu"]');
    expect(profile.attributes('disabled')).toBeDefined(); expect(menu.attributes('disabled')).toBeDefined();
    expect(buttonByLabel(f.wrapper, t('automations.templates.open')).attributes('disabled')).toBeDefined();
    expect(f.wrapper.get('[data-test="library-loading"]').text()).toContain('Loading saved profiles');
    expect(f.calls.some((call) => call.method === 'automation.library.submit')).toBe(false);
    await f.wrapper.get('[data-test="library-retry-loading"]').trigger('click'); await flushPromises();
    blocked = false; reject(new Error('initial snapshot unavailable')); await f.initializing; await flushPromises();
    expect(f.wrapper.text()).toContain('initial snapshot unavailable');
    expect(menu.attributes('disabled')).toBeDefined();
    await f.wrapper.get('[data-test="library-retry-loading"]').trigger('click'); await flushPromises();
    expect(profile.attributes('disabled')).toBeUndefined(); expect(menu.attributes('disabled')).toBeUndefined();
    expect(buttonByLabel(f.wrapper, t('automations.templates.open')).attributes('disabled')).toBeUndefined();
    expect(f.wrapper.find('[data-test="library-loading"]').exists()).toBe(false);
    expect(f.wrapper.text()).not.toContain('initial snapshot unavailable');
    expect(f.calls.some((call) => call.method === 'automation.library.submit' || call.method === 'app.state.set')).toBe(false);
    await menu.trigger('click'); await nextTick();
    Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]')).find((entry) => entry.textContent?.trim() === t('automations.profile.create'))!.click();
    await flushPromises();
    await f.wrapper.get('input[maxlength="80"]').setValue('Ready profile');
    documentButton(t('automations.profile.createConfirm')).click(); await flushPromises();
    const library = libraryJournal(f.client);
    expect(library.draft.value?.context.definitions.actions[0]?.name).toBe('Log gift');
    expect(library.draft.value?.mutation).toMatchObject({ operation: 'create-profile', name: 'Ready profile' });
    library.dismiss(); await flushPromises();
    f.automation.dispose();
  });

  it('keeps the runtime variables as their own tab next to events and actions', async () => {
    const { wrapper, calls } = await mountPage((method) => (method === 'globals.list' ? { globals: { commandPort: '46665' } } : undefined));
    expect(wrapper.findAll('[role="tab"]').map((tab) => tab.text())).toEqual(['Events (2)', 'Actions (1)', t('automations.library.variables')]);
    expect(wrapper.find('.globals-editor').exists()).toBe(false);
    expect(calls.some((call) => call.method === 'globals.list')).toBe(false);
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === t('automations.library.variables'))!.trigger('click');
    await flushPromises();
    expect(wrapper.findAll('.globals-editor input').map((input) => (input.element as HTMLInputElement).value)).toEqual(['commandPort', '46665']);
    expect(wrapper.find('.automation-library__table').exists()).toBe(false);
    expect(wrapper.get('.automations-page__runs').isVisible()).toBe(true);
  });

  it('keeps an unsaved variable draft while switching tabs and saves it through the same contract', async () => {
    const { wrapper, calls } = await mountPage((method) => (method === 'globals.list' ? { globals: { commandPort: '46665' } } : method === 'globals.set' ? { ok: true } : undefined));
    const tab = (name: string) => wrapper.findAll('[role="tab"]').find((item) => item.text().startsWith(name))!;
    await tab(t('automations.library.variables')).trigger('click');
    await flushPromises();
    await wrapper.findAll('.globals-editor input')[1]!.setValue('25575');
    await tab(t('automations.library.actions')).trigger('click');
    await tab(t('automations.library.variables')).trigger('click');
    expect((wrapper.findAll('.globals-editor input')[1]!.element as HTMLInputElement).value).toBe('25575');
    await buttonByLabel(wrapper, t('globals.save')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'globals.set')?.params).toEqual({ key: 'commandPort', value: '25575' });
  });

  it('opens on the variables from the old Settings link', async () => {
    const { wrapper } = await mountPage((method) => (method === 'globals.list' ? { globals: { a: '1' } } : undefined), true, { query: '?view=variables' });
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe(t('automations.library.variables'));
    expect(wrapper.find('.globals-editor').exists()).toBe(true);
  });

  it('has no Cloud tab in the library: the Cloud library is a tab of the templates dialog', async () => {
    const { wrapper } = await mountPage();
    const library = () => wrapper.findAll('[role="tab"]').map((tab) => tab.text());
    expect(library()).toEqual(['Events (2)', 'Actions (1)', t('automations.library.variables')]);
    const unified = await mountPage(undefined, true, { announce: ['catalog.items'] });
    expect(unified.wrapper.findAll('[role="tab"]').map((tab) => tab.text())).toEqual(['Events (2)', 'Actions (1)', t('automations.library.variables')]);
    expect(document.querySelector('dialog[open]')).toBeNull();
  });

  it('opens the templates dialog on its Cloud tab from a saved ?view=cloud link and clears the link when it closes', async () => {
    const { wrapper, router } = await mountPage(undefined, true, { query: '?view=cloud' });
    await flushPromises();
    const dialog = document.querySelector('dialog[open]')!;
    expect(dialog).not.toBeNull();
    const tabs = Array.from(dialog.querySelectorAll('[role="tab"]'));
    expect(tabs.map((tab) => tab.textContent?.trim())).toEqual([t('importDialog.templatesTab'), t('importDialog.cloudTab'), t('importDialog.manualTab')]);
    expect(tabs.find((tab) => tab.getAttribute('aria-selected') === 'true')?.textContent?.trim()).toBe(t('importDialog.cloudTab'));
    expect(dialog.textContent).toContain(t('overlayLibrary.library'));
    // The page is still the library: no Cloud tab in its own bar.
    expect(wrapper.findAll('.automation-library [role="tab"]').map((tab) => tab.text())).toEqual(['Events (2)', 'Actions (1)', t('automations.library.variables')]);
    expect(router.currentRoute.value.query.view).toBe('cloud');
    dialog.querySelector<HTMLButtonElement>('button[aria-label="Close"]')!.click();
    await flushPromises();
    expect(router.currentRoute.value.query.view).toBeUndefined();
    wrapper.unmount();
  });

  it('opens the templates dialog on Templates, not on a Cloud tab left from before', async () => {
    const { wrapper } = await mountPage();
    await buttonByLabel(wrapper, t('automations.templates.open')).trigger('click');
    await flushPromises();
    const selected = document.querySelector('dialog[open] [role="tab"][aria-selected="true"]');
    expect(selected?.textContent?.trim()).toBe(t('importDialog.templatesTab'));
    wrapper.unmount();
  });

  it('lists events and opens the editor', async () => {
    const { wrapper } = await mountPage();
    const rows = wrapper.findAll('.automation-library tbody tr');
    expect(rows.map((row) => row.text())).toEqual([expect.stringContaining('Big gifts'), expect.stringContaining('Chat hello')]);
    expect(wrapper.find('.event-editor').exists()).toBe(false);
    await selectRow(wrapper, 'Big gifts');
    expect(wrapper.get('.event-editor .base-panel__title').text()).toBe('Big gifts');
    // The old one-sentence summary only repeated the fields below it.
    expect(wrapper.find('.event-editor__summary').exists()).toBe(false);
    // One navigation area: the open record replaces the library until you go back.
    expect(wrapper.find('.automation-library').exists()).toBe(false);
    await goBack(wrapper);
    expect(wrapper.find('.event-editor').exists()).toBe(false);
    expect(wrapper.findAll('.automation-library tbody tr')).toHaveLength(2);
  });
