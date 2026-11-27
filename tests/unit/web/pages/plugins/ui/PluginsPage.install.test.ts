import { describe, flushPromises, expect, it, t, mountPage, buttonByText, buttonByLabel, openTab, selectPlugin, detailsTitle } from '../../../../../../src/web/pages/plugins/ui/plugins-page-fixture.ts';
import { PLUGIN_ID, RPC } from '../../../../../shared/index.ts';

describe('PluginsPage install', () => {
  it('reviews a package file and requires sideload consent before installing', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByLabel(wrapper, t('plugins.install.package')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.pluginsPackagesInspect)?.params).toEqual({
      spec: { kind: 'file', path: 'C:/x.plugin' },
    });
    expect(wrapper.text()).toContain('Install X Demo');
    expect(wrapper.text()).toContain(t('packages.access.audio_output'));
    expect(wrapper.text()).toContain(t('packages.signature.unsigned'));
    expect(buttonByText(wrapper, t('plugins.details.install')).attributes('disabled')).toBeDefined();
    await wrapper.findAll('input[type="checkbox"]').at(-1)!.setValue(true);
    await buttonByText(wrapper, t('plugins.details.install')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.pluginsPackagesInstall)?.params).toEqual({
      spec: { kind: 'file', path: 'C:/x.plugin' },
      consentToken: 'token-1',
    });
  });

  it('reviews registry plugins from the browse tab', async () => {
    const { wrapper, calls } = await mountPage();
    await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
    await selectPlugin(wrapper, 'Acme TTS');
    await flushPromises();
    expect(calls.find((call) => call.method === 'registry.package')?.params).toEqual({ id: PLUGIN_ID.acmeTts, registry: 'official' });
    expect(wrapper.text()).toContain('Reads chat aloud.');
    await buttonByText(wrapper, t('packages.remote.install')).trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === RPC.pluginsPackagesInspect).at(-1)?.params).toEqual({
      spec: { kind: 'registry', id: PLUGIN_ID.acmeTts, version: '1.2.0', registry: 'official' },
    });
  });

  it('blocks remote review when current status is missing or the key is revoked', async () => {
    for (const [liveStatuses, expected] of [
      [{}, t('packages.liveStatus.unknown')],
      [{ '1.2.0': 'key-revoked' }, t('packages.liveStatus.key-revoked')],
    ] as const) {
      // The previous round's saved selection would reopen its details instead of the tabs.
      localStorage.clear();
      const { wrapper, calls } = await mountPage(undefined, undefined, liveStatuses);
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      await selectPlugin(wrapper, 'Acme TTS');
      await flushPromises();
      expect(wrapper.text()).toContain(expected);
      expect(buttonByText(wrapper, t('packages.remote.install')).attributes('disabled')).toBeDefined();
      await buttonByText(wrapper, t('packages.remote.install')).trigger('click');
      expect(calls.some((call) => call.method === RPC.pluginsPackagesInspect)).toBe(false);
      wrapper.unmount();
    }
  });

  it('explains that the marketplace is unpublished when no registry is available', async () => {
    const { wrapper, calls } = await mountPage(undefined, []);
    await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
    expect(wrapper.text()).toContain(t('plugins.catalog.marketplaceUnavailable'));
    expect(wrapper.find('input[type="search"]').exists()).toBe(false);
    expect(calls.map((call) => call.method)).toContain('registry.list');
  });

  it('shows processor metrics and runs a test', async () => {
    const { wrapper, calls } = await mountPage();
    await openTab(wrapper, t('plugins.views.processors'));
    expect(calls.map((call) => call.method)).toContain('processors.status');
    const entry = wrapper.get('.processors__row');
    expect(entry.text()).toContain('enrich');
    expect(entry.text()).toContain('DECK');
    // A healthy processor needs no status in the list; its detail has one.
    expect(entry.text()).not.toContain(t('plugins.progress.ready'));
    await entry.get('.processors__open').trigger('click');
    // The detail replaces the list and the tabs, with a way back.
    expect(wrapper.find('.processors').exists()).toBe(false);
    expect(wrapper.find('[role="tablist"]').exists()).toBe(false);
    expect(detailsTitle(wrapper)).toBe('enrich');
    expect(wrapper.get('.base-panel__status').text()).toContain(t('plugins.progress.ready'));
    expect(wrapper.text()).toContain('boom');
    await buttonByText(wrapper, t('plugins.processors.test')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'processors.test')?.params).toMatchObject({ pluginId: 'deck', processorId: 'enrich' });
    expect(wrapper.text()).toContain('Succeeded in 3 ms');
    await buttonByLabel(wrapper, t('common.back')).trigger('click');
    expect(wrapper.find('.processors__row').exists()).toBe(true);
  });
});
