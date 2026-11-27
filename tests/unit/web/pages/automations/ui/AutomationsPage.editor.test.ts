import { t, mountPage, buttonByText, buttonByLabel, goBack, documentButton, selectRow, it, expect, vi, nextTick, flushPromises, afterEach, beforeEach, resetResolutionPreference } from '../../../../../../src/web/pages/automations/ui/automations-page-fixture.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { ACTION_TYPE, EVENT_KIND, RPC } from '../../../../../shared/index.ts';
const itn = (key: string, params?: Record<string, string>) => i18n.global.t(key, params ?? {});

beforeEach(() => { localStorage.clear(); resetResolutionPreference('manual'); });
afterEach(() => resetResolutionPreference('automatic'));


it('offers the sync panel for the events and for the actions, but not on the variables', async () => {
    const { wrapper } = await mountPage();
    const tab = (name: string) => wrapper.findAll('[role="tab"]').find((item) => item.text().startsWith(name))!;
    const button = () => wrapper.find('button[aria-label^="Sync changes"]');
    expect(button().exists()).toBe(true);
    await button().trigger('click');
    await flushPromises();
    expect(wrapper.get('dialog h2').text()).toBe('Changes · Events');
    await wrapper.get('dialog button[aria-label="Close"]').trigger('click');
    await tab(t('automations.library.actions')).trigger('click');
    await flushPromises();
    await button().trigger('click');
    await flushPromises();
    expect(wrapper.get('dialog h2').text()).toBe('Changes · Actions');
    await wrapper.get('dialog button[aria-label="Close"]').trigger('click');
    await tab(t('automations.library.variables')).trigger('click');
    await flushPromises();
    expect(button().exists()).toBe(false);
  });

  it('sorts the library from the filter menu', async () => {
    const { wrapper } = await mountPage();
    const filter = wrapper.get('.automation-library__sort');
    expect(filter.classes()).not.toContain('automation-library__sort--active');
    await filter.trigger('click');
    await nextTick();
    const item = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitemcheckbox"]'))
      .find((entry) => entry.textContent?.trim() === t('automations.library.sorts.name-desc'))!;
    item.click();
    await flushPromises();
    expect(wrapper.findAll('.automation-library tbody tr').map((row) => row.text())).toEqual([
      expect.stringContaining('Chat hello'),
      expect.stringContaining('Big gifts'),
    ]);
    expect(wrapper.get('.automation-library__sort').classes()).toContain('automation-library__sort--active');
  });

  it('toggles a record from the table', async () => {
    const { wrapper, calls, requests } = await mountPage();
    await wrapper.findAll('.automation-library tbody tr')[1]!.get('input[type="checkbox"]').setValue(false);
    expect(calls.some((call) => call.method === RPC.automationDisable)).toBe(false);
    expect(document.body.textContent).toContain('e2 · version 1');
    documentButton(t('automations.conflict.confirmToggle')).click();
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.automationDefinitionSubmit)?.params).toMatchObject({ operation: 'disable', id: 'e2', kind: 'event', expectedRevision: '1' });
    expect(requests.find((call) => call.method === RPC.automationDisable)?.meta).toEqual({ expectedRevision: '1' });
  });

  it('creates and saves a new event', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, t('automations.library.newEvent')).trigger('click');
    await wrapper.get('.event-editor input[type="text"]').setValue('Followers');
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.automationDefinitionSubmit)?.params).toMatchObject({ operation: 'create',
      kind: 'event',
      record: { name: 'Followers', trigger: EVENT_KIND.tiktokGift },
    });
  });

  it('picks an action type and saves the new action', async () => {
    const { wrapper, calls } = await mountPage();
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text().startsWith(t('automations.library.actions')))!.trigger('click');
    await buttonByText(wrapper, t('automations.library.newAction')).trigger('click');
    await nextTick();
    Array.from(document.querySelectorAll<HTMLButtonElement>('.type-picker__card'))[0]!.click();
    await nextTick();
    expect(wrapper.find('.action-editor').exists()).toBe(true);
    await buttonByText(wrapper, t('automations.editor.save')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.automationDefinitionSubmit)?.params).toMatchObject({ operation: 'create',
      kind: 'action',
      record: { typeId: ACTION_TYPE.coreLog, name: 'Write to log' },
    });
  });

  it('does not ask to discard an untouched new event', async () => {
    const { wrapper } = await mountPage();
    await buttonByText(wrapper, t('automations.library.newEvent')).trigger('click');
    await flushPromises();
    expect(wrapper.text()).not.toContain(t('automations.editor.unsaved'));
    await goBack(wrapper);
    expect(document.body.textContent).not.toContain(t('automations.editor.discardTitle'));
  });

  it('duplicates and deletes events from their library row menu', async () => {
    const { wrapper, calls } = await mountPage();
    const kebab = (name: string) => wrapper.get(`button[aria-label="${itn('automations.library.rowActions', { name })}"]`);
    const item = (key: string) => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
      .find((entry) => entry.textContent?.trim() === t(key))!;

    await kebab('Big gifts').trigger('click');
    await nextTick();
    item('automations.library.duplicate').click();
    await flushPromises();
    const created = calls.find((call) => call.method === RPC.automationDefinitionSubmit && call.params?.operation === 'create');
    const copy = created?.params?.record as { id?: string; name?: string } | undefined;
    expect(copy).toMatchObject({ name: itn('automations.library.duplicateName', { name: 'Big gifts' }) });
    expect(copy?.id).not.toBe('e1');

    await kebab('Chat hello').trigger('click');
    await nextTick();
    item('automations.library.delete').click();
    await flushPromises();
    expect(document.body.textContent).toContain(itn('automations.library.deleteTitle', { name: 'Chat hello' }));
    documentButton(t('automations.library.delete')).click();
    await flushPromises();
    expect(calls.some((call) => call.method === RPC.automationDefinitionSubmit && call.params?.operation === 'delete' && call.params?.id === 'e2')).toBe(true);
  });

  it('asks before leaving unsaved changes', async () => {
    const { wrapper } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    await wrapper.get('.event-editor input[type="text"]').setValue('Renamed');
    await goBack(wrapper);
    expect(document.body.textContent).toContain(t('automations.editor.discardTitle'));
    documentButton(t('automations.editor.discardConfirm')).click();
    await nextTick();
    await selectRow(wrapper, 'Chat hello');
    expect(wrapper.get('.event-editor .base-panel__title').text()).toBe('Chat hello');
  });

  it('confirms deleting an event', async () => {
    const { wrapper, calls } = await mountPage();
    await selectRow(wrapper, 'Chat hello');
    await buttonByText(wrapper, t('automations.editor.delete')).trigger('click');
    await nextTick();
    documentButton(t('automations.editor.deleteConfirm')).click();
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.automationDefinitionSubmit)?.params).toMatchObject({ operation: 'delete', id: 'e2', kind: 'event', expectedRevision: '1' });
  });

  it('applies a built-in rule template', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByLabel(wrapper, t('automations.templates.open')).trigger('click');
    await nextTick();
    const card = Array.from(document.querySelectorAll<HTMLButtonElement>('.rule-templates__card')).find((entry) =>
      entry.textContent?.includes(t('automations.builtinTemplates.chatPointsTitle')),
    )!;
    card.click();
    await nextTick();
    documentButton(t('automations.templates.create')).click();
    await flushPromises();
    expect(calls.some((call) => call.method === RPC.automationLibrarySubmit)).toBe(false);
    // The confirm action is an icon button: it is found by its aria-label.
    Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.getAttribute('aria-label') === t('libraryRecovery.confirm'))!.click(); await flushPromises();
    const created = calls.filter((call) => call.method === RPC.automationLibrarySubmit);
    expect(created).toHaveLength(1);
    expect(created[0]?.params?.mutation).toMatchObject({ operation: 'apply-template', profileId: 'default', entry: { event: { name: expect.any(String) }, actions: [{ name: expect.any(String) }] } });
    expect(calls.some((call) => call.method === RPC.automationCreate)).toBe(false);
  });

  it('explains the grouped run menu on hover', async () => {
    const { wrapper } = await mountPage();
    await selectRow(wrapper, 'Big gifts');
    vi.useFakeTimers();
    try {
      const runMenu = buttonByText(wrapper, t('automations.runMenu.label'));
      runMenu.element.dispatchEvent(new Event('focusin', { bubbles: true }));
      vi.advanceTimersByTime(600);
      expect(document.getElementById('app-tooltip')?.textContent).toBe(
        t('automations.runMenu.hint'),
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it('shows one type icon per action row', async () => {
    const { wrapper } = await mountPage();
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text().startsWith(t('automations.library.actions')))!.trigger('click');
    await nextTick();
    expect(wrapper.findAll('.automation-library tbody tr')).toHaveLength(1);
    expect(wrapper.findAll('.automation-library__type-icon')).toHaveLength(1);
  });

  it('shows recent runs', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('.automation-runs').text()).toContain('Log gift');
    expect(wrapper.get('.automation-runs').text()).toContain('logged');
  });

  it('folds the recent runs away and back', async () => {
    const { wrapper } = await mountPage();
    await buttonByLabel(wrapper, t('automations.runs.collapse')).trigger('click');
    expect(wrapper.get('.automation-runs').text()).not.toContain('Log gift');
    await buttonByLabel(wrapper, t('automations.runs.expand')).trigger('click');
    expect(wrapper.get('.automation-runs').text()).toContain('Log gift');
  });

  it('restores the folded recent runs from storage', async () => {
    const { wrapper } = await mountPage();
    await buttonByLabel(wrapper, t('automations.runs.collapse')).trigger('click');
    expect(JSON.parse(localStorage.getItem('tiktok-live-view-state')!).automationRuns).toEqual({ open: false });
    wrapper.unmount();
    const { wrapper: remounted } = await mountPage();
    expect(remounted.get('.automation-runs').text()).not.toContain('Log gift');
    await buttonByLabel(remounted, t('automations.runs.expand')).trigger('click');
    expect(remounted.get('.automation-runs').text()).toContain('Log gift');
  });

  it('opens the editor as a full-screen sheet with a back button on narrow screens', async () => {
    vi.stubGlobal('matchMedia', () => ({
      matches: true,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
    }));
    try {
      const { wrapper } = await mountPage();
      expect(wrapper.find('.automations-page__sheet').exists()).toBe(false);
      await selectRow(wrapper, 'Big gifts');
      const sheet = wrapper.find('.automations-page__sheet');
      expect(sheet.exists()).toBe(true);
      expect(sheet.attributes('role')).toBe('dialog');
      expect(sheet.get('.base-panel__title').text()).toBe('Big gifts');
      expect(wrapper.find('.automations-page__sheet .event-editor').exists()).toBe(true);
      // On narrow screens the library stays underneath the sheet.
      expect(wrapper.find('.automation-library').exists()).toBe(true);
      await goBack(wrapper);
      await nextTick();
      expect(wrapper.find('.automations-page__sheet').exists()).toBe(false);
      expect(wrapper.find('.automation-library').exists()).toBe(true);
    } finally {
      vi.unstubAllGlobals();
    }
  });
