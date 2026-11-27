import {
  describe,
  flushPromises,
  expect,
  it,
  vi,
  nextTick,
  t,
  plugin,
  mountPage,
  buttonByText,
  buttonByLabel,
  tabNames,
  openTab,
  selectPlugin,
  openItemMenu,
  detailsTitle,
  menuItems,
  menuItem,
} from '../../../../../../src/web/pages/plugins/ui/plugins-page-fixture.ts';
import { EVENT_KIND, RPC, STORAGE_KEY } from '../../../../../shared/index.ts';

describe('PluginsPage list', () => {
  it('saves the current view on leaving and cannot overwrite a later view from a delayed callback', async () => {
    vi.useFakeTimers();
    try {
      const { wrapper } = await mountPage();
      await selectPlugin(wrapper, 'DECK');
      wrapper.unmount();
      const saved = JSON.parse(localStorage.getItem(STORAGE_KEY.viewState)!);
      expect(saved.plugins.selected).toBe('deck');
      saved.plugins.selected = 'next-screen';
      localStorage.setItem(STORAGE_KEY.viewState, JSON.stringify(saved));
      await vi.advanceTimersByTimeAsync(500);
      expect(JSON.parse(localStorage.getItem(STORAGE_KEY.viewState)!).plugins.selected).toBe('next-screen');
    } finally {
      vi.useRealTimers();
    }
  });
  it('opens the plugin preferences inside the same panel from the shared actions menu and goes back', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.find('.plugin-preferences').exists()).toBe(false);
    await buttonByLabel(wrapper, t('syncChanges.more')).trigger('click');
    menuItem(t('plugins.catalog.preferences')).click();
    await flushPromises();
    expect(wrapper.find('.plugin-catalog').exists()).toBe(false);
    expect(wrapper.get('.plugin-preferences .base-panel__title').text()).toBe(t('plugins.catalog.preferences'));
    expect(wrapper.get('.plugin-preferences').text()).toContain(t('packages.sources.allowSideload'));
    expect(wrapper.find('#plugin-update-policy-channel').exists()).toBe(true);
    await buttonByLabel(wrapper, t('common.back')).trigger('click');
    expect(wrapper.find('.plugin-preferences').exists()).toBe(false);
    expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
  });

  it('opens the preferences straight from the legacy Settings link', async () => {
    const { wrapper, router } = await mountPage(undefined, undefined, undefined, [], [], '?view=preferences');
    expect(wrapper.find('.plugin-preferences').exists()).toBe(true);
    await buttonByLabel(wrapper, t('common.back')).trigger('click');
    await flushPromises();
    expect(router.currentRoute.value.query).toEqual({});
    expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
  });

  it('opens plugin details in the same panel and returns to the list, on narrow screens too', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: () => {}, removeEventListener: () => {} }));
    try {
      const { wrapper } = await mountPage();
      await selectPlugin(wrapper, 'DECK');
      expect(detailsTitle(wrapper)).toBe('DECK');
      expect(wrapper.find('.plugin-catalog').exists()).toBe(false);
      await buttonByLabel(wrapper, t('common.back')).trigger('click');
      expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
      expect(wrapper.find('.plugin-details').exists()).toBe(false);
      await selectPlugin(wrapper, 'DECK');
      expect(detailsTitle(wrapper)).toBe('DECK');
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('shows one navigation area with icon tabs, a search field and icon tools', async () => {
    const { wrapper } = await mountPage();
    expect(tabNames(wrapper)).toEqual([t(RPC.pluginsCatalogBrowse), 'Installed 1', t('plugins.catalog.updates'), t('plugins.views.processors')]);
    expect(wrapper.find('.plugins-page__toolbar').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('Plugins bring an external dependency');
    expect(wrapper.get('h2').text()).toBe(t('navigation.plugins'));
    expect(wrapper.get('input[type="search"]').attributes('aria-label')).toBe(t('plugins.catalog.search'));
    expect(buttonByLabel(wrapper, t('plugins.install.package')).exists()).toBe(true);
    expect(buttonByLabel(wrapper, t('plugins.catalog.checkUpdates')).exists()).toBe(false);
    // No master/detail, so no placeholder waiting for a selection either.
    expect(wrapper.text()).not.toContain(t('plugins.details.empty'));
    expect(wrapper.find('.master-detail').exists()).toBe(false);
  });

  it('has a sync button in the tools that explains why it is disabled while signed out', async () => {
    const { wrapper } = await mountPage();
    const sync = wrapper.get('button[aria-label^="Sync changes"]');
    expect(sync.attributes('disabled')).toBeDefined();
    expect(sync.attributes('aria-description')).toBe(t('syncChanges.signedOut'));
  });

  it('lists installed plugins with a switch and counters, and flags only the ones that cannot load', async () => {
    const broken = plugin(
      'hotkeys',
      { available: false, unavailableReason: 'No input access' },
      { eventTypeIds: [EVENT_KIND.hotkeyPressed], actionTypeIds: [] },
    );
    const { wrapper, calls } = await mountPage([plugin('deck'), broken]);
    const rows = wrapper.findAll('.plugin-catalog__row');
    expect(rows).toHaveLength(2);
    expect(rows[0]!.text()).toContain('Plugin package · 1.2.0');
    expect(rows[0]!.get('[role="img"][aria-label="Actions: 1"]').text()).toBe('1');
    expect(rows[1]!.get('[role="img"][aria-label="Events: 1"]').text()).toBe('1');
    expect(rows[0]!.text()).not.toContain(t('plugins.catalog.unavailable'));
    expect(rows[1]!.text()).toContain(t('plugins.catalog.unavailable'));
    // One status per fact: the switch says enabled, so no "Active" text beside it.
    expect(wrapper.text()).not.toContain('Active');
    await rows[0]!.get('input[role="switch"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });
    expect(rows[1]!.find('input[role="switch"][aria-label="Enable HOTKEYS"]').exists()).toBe(true);
  });

  it('lists installed plugins apart from bundled and registry plugins', async () => {
    const { wrapper, calls } = await mountPage();
    expect(wrapper.findAll('.plugin-catalog__row').map((item) => item.text())).toEqual([expect.stringContaining('DECK')]);
    await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
    expect(calls.map((call) => call.method)).toContain('registry.search');
    const rows = wrapper.findAll('.plugin-catalog__row');
    expect(rows.map((item) => item.text())).toEqual([expect.stringContaining('TTS'), expect.stringContaining('Acme TTS')]);
    expect(rows[0]!.find('input[role="switch"]').exists()).toBe(false);
    expect(buttonByLabel(wrapper, 'Install Acme TTS').exists()).toBe(true);
  });

  it('lists the updates with the versions and updates one with its icon button', async () => {
    const update = {
      id: 'deck',
      registry: 'official',
      current: '1.2.0',
      latest: '1.3.0',
      channel: 'stable',
      pinned: false,
      permissionDiff: {},
      autoApplicable: false,
      currentStatus: 'approved',
      latestStatus: 'approved',
    };
    const { wrapper, calls } = await mountPage(undefined, undefined, undefined, [], [update]);
    expect(tabNames(wrapper)).toContain('Updates 1');
    await openTab(wrapper, t('plugins.catalog.updates'));
    expect(wrapper.get('.plugin-catalog__row').text()).toContain('1.2.0 → 1.3.0');
    expect(buttonByLabel(wrapper, t('plugins.catalog.checkUpdates')).exists()).toBe(true);
    expect(buttonByLabel(wrapper, t('plugins.catalog.updateAll')).attributes('disabled')).toBeUndefined();
    await buttonByLabel(wrapper, 'Update DECK').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.pluginsPackagesUpdate)?.params).toEqual({ pluginId: 'deck', version: '1.3.0' });
  });

  it('filters the catalog by search', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await wrapper.get('input[type="search"]').setValue('scenes');
    expect(wrapper.findAll('.plugin-catalog__row')).toHaveLength(1);
    await wrapper.get('input[type="search"]').setValue('nothing');
    expect(wrapper.text()).toContain('No plugins match “nothing”.');
  });

  it('shows details and toggles the plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    expect(detailsTitle(wrapper)).toBe('DECK');
    expect(wrapper.get('.plugin-details .base-panel__status').text()).toBe('1.2.0');
    expect(wrapper.find('.base-markdown strong').text()).toBe('deck');
    expect(wrapper.text()).toContain('Switch scene');
    // The version is in the header, not repeated among the facts.
    expect(wrapper.text().match(/1\.2\.0/g)).toHaveLength(1);
    await wrapper.get('.plugin-details input[role="switch"]').setValue(false);
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });
  });

  it('offers only the detail tabs the plugin has use for, each with its icon', async () => {
    const server = plugin('srv', {}, { hasSettings: true, hasConnectionProbe: true });
    const builtin = plugin('core', {}, { source: 'builtin' });
    const { wrapper } = await mountPage([server, builtin]);
    await selectPlugin(wrapper, 'SRV');
    expect(tabNames(wrapper)).toEqual([
      t('plugins.details.tabs.general'),
      t('plugins.details.tabs.settings'),
      t('plugins.details.tabs.connection'),
      t('plugins.details.tabs.versions'),
    ]);
    expect(wrapper.findAll('[role="tab"] svg')).toHaveLength(4);
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === t('plugins.details.tabs.connection'))!.trigger('click');
    // The same editor the Connections page uses, with its own status and test button.
    expect(wrapper.find('.connection-editor--sections').exists()).toBe(true);
    expect(wrapper.text()).toContain(t('connections.servers.notConfigured'));
    await buttonByLabel(wrapper, t('common.back')).trigger('click');
    await selectPlugin(wrapper, 'CORE');
    expect(tabNames(wrapper)).toEqual([t('plugins.details.tabs.general')]);
  });

  it('lays the general tab out as origin and permissions beside what the plugin adds', async () => {
    const { wrapper } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    const columns = wrapper.findAll('.plugin-details__column');
    expect(columns).toHaveLength(2);
    expect(columns[0]!.text()).toContain(t('plugins.details.sources.user'));
    expect(columns[0]!.text()).toContain('Local server');
    expect(columns[0]!.text()).toContain('network');
    expect(columns[1]!.text()).toContain('Switch scene');
  });

  it('explains in the open plugin why it cannot load, once', async () => {
    const broken = plugin('hotkeys', { available: false, unavailableReason: 'No input access' });
    const { wrapper } = await mountPage([broken]);
    await selectPlugin(wrapper, 'HOTKEYS');
    expect(wrapper.get('[role="alert"]').text()).toBe('This plugin cannot load on this machine: No input access');
    expect(wrapper.text().match(/No input access/g)).toHaveLength(1);
  });

  it('confirms uninstall with the action usage', async () => {
    const { wrapper, calls } = await mountPage();
    await selectPlugin(wrapper, 'DECK');
    await buttonByLabel(wrapper, t('plugins.details.uninstall')).trigger('click');
    await nextTick();
    expect(document.body.textContent).toContain('2 actions use it and will stop working.');
    const confirm = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find(
      (button) => button.textContent?.trim() === t('plugins.details.uninstallConfirm'),
    )!;
    confirm.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.uninstall')?.params).toEqual({ pluginId: 'deck', purgeData: false });
  });

  it('offers plugin commands from the context menu on a list item', async () => {
    const { wrapper, calls } = await mountPage();
    await openItemMenu(wrapper, 'DECK');
    expect(menuItems().map((item) => item.textContent?.trim())).toEqual([
      t('plugins.menu.open'),
      t('plugins.details.enabled'),
      t('plugins.details.tabs.settings'),
      t('plugins.details.tabs.connection'),
      t('plugins.details.tabs.versions'),
      t('plugins.settings.test'),
      t('plugins.details.uninstall'),
    ]);
    menuItem(t('plugins.details.enabled')).click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.disable')?.params).toEqual({ pluginId: 'deck' });

    await openItemMenu(wrapper, 'DECK');
    menuItem(t('plugins.details.uninstall')).click();
    await flushPromises();
    expect(detailsTitle(wrapper)).toBe('DECK');
    expect(document.body.textContent).toContain('2 actions use it and will stop working.');
    Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find(
      (button) => button.textContent?.trim() === t('plugins.details.uninstallConfirm'),
    )!.click();
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.uninstall')?.params).toEqual({ pluginId: 'deck', purgeData: false });
  });

  it('opens a details tab from the context menu', async () => {
    const { wrapper } = await mountPage([plugin('deck'), plugin('vts')]);
    await openItemMenu(wrapper, 'VTS');
    menuItem(t('plugins.details.tabs.versions')).click();
    await flushPromises();
    expect(detailsTitle(wrapper)).toBe('VTS');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe(t('plugins.details.tabs.versions'));
  });

  it('removes non-package plugins from the installed list instead of uninstalling', async () => {
    const dev = plugin('devkit', { available: false, enabled: false }, { source: 'development' });
    const { wrapper, calls } = await mountPage([dev]);
    await selectPlugin(wrapper, 'DEVKIT');
    expect(buttonByLabel(wrapper, t('plugins.details.uninstall')).exists()).toBe(false);
    await buttonByLabel(wrapper, t('plugins.menu.remove')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'devkit', installed: false });

    await buttonByLabel(wrapper, t('common.back')).trigger('click');
    await openItemMenu(wrapper, 'DEVKIT');
    expect(menuItems().map((item) => item.textContent?.trim())).toContain(t('plugins.menu.remove'));
    expect(menuItems().map((item) => item.textContent?.trim())).not.toContain(t('plugins.details.uninstall'));
  });

  it('activates a bundled plugin', async () => {
    const { wrapper, calls } = await mountPage();
    await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
    await selectPlugin(wrapper, 'TTS');
    await buttonByText(wrapper, t('plugins.details.activate')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'plugins.install.set')?.params).toEqual({ pluginId: 'tts', installed: true });
  });
});
