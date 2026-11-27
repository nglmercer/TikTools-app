import {
  type VueWrapper,
  flushPromises,
  beforeEach,
  describe,
  expect,
  it,
  ControlCallError,
  type PluginStatus,
  t,
  mountPage,
  buttonByLabel,
  openTab,
  detailsTitle,
  menuItem,
  REGISTRY_HOST,
} from '../../../../../../src/web/pages/plugins/ui/plugins-page-fixture.ts';
import { FIXTURE_ID, PLUGIN_ID, RPC, STORAGE_KEY } from '../../../../../shared/index.ts';

describe('PluginsPage catalog', () => {
  describe('on a host that announces the unified catalog', () => {
    const unified = (plugins?: PluginStatus[]) => mountPage(plugins, undefined, undefined, [RPC.catalogItems]);
    const savedView = () => JSON.parse(localStorage.getItem(STORAGE_KEY.viewState) ?? '{}').plugins?.view;

    // A previous test's debounced save must not land in this test's storage.
    beforeEach(async () => {
      await new Promise((resolve) => setTimeout(resolve, 350));
      localStorage.clear();
    });

    it('still opens on the installed plugins and never requests the Cloud catalog', async () => {
      const { wrapper, calls } = await unified();
      expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
      expect(wrapper.find('.explore-toolbar').exists()).toBe(false);
      expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toContain(t('packages.versions.current'));
      expect(wrapper.find('button[aria-label="Cloud catalog"]').exists()).toBe(false);
      expect(calls.some((call) => call.method === RPC.catalogItems)).toBe(false);
    });

    it('sends a saved catalog or Cloud view to the installed plugins', async () => {
      for (const view of ['catalog', 'cloud']) {
        localStorage.setItem(STORAGE_KEY.viewState, JSON.stringify({ version: 1, plugins: { view } }));
        const { wrapper } = await unified();
        expect(wrapper.find('.plugin-catalog').exists()).toBe(true);
        expect(wrapper.find('.explore-toolbar').exists()).toBe(false);
      }
    });

    it('does not write the implicit view as a choice', async () => {
      const { wrapper } = await unified();
      expect(savedView()).toBeUndefined();
      await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === t('plugins.views.processors'))!.trigger('click');
      await new Promise((resolve) => setTimeout(resolve, 400));
      expect(savedView()).toBe('processors');
    });
  });

  describe('Explore on the Cloud catalog', () => {
    const wire = (kind: string, id: string, title: string) => ({
      identity: { kind, id },
      presentation: { title, summary: '', publisher: 'acme', categories: [] },
      compatibility: { targets: [] },
      releaseRef: kind === 'plugin' ? { type: 'pluginRelease', version: '1.2.0' } : { type: 'documentRevision', revision: 3, sha256: 'a'.repeat(64) },
      publication: { yanked: false },
    });
    const cloudItems = [
      wire('plugin', PLUGIN_ID.acmeTts, 'Acme TTS'),
      wire('template', FIXTURE_ID.tpl1, 'Gift alert'),
      wire('profile', FIXTURE_ID.prof1, 'Starter pack'),
    ];
    const explore = (handle?: Parameters<typeof mountPage>[6], announce: string[] = [RPC.catalogItems]) =>
      mountPage(
        undefined,
        undefined,
        undefined,
        announce,
        [],
        '',
        handle ?? ((method) => (method === RPC.catalogItems ? { items: cloudItems, nextCursor: null } : undefined)),
      );
    const names = (wrapper: VueWrapper) => wrapper.findAll('.explore-list__row').map((item) => item.get('.explore-list__name').text());
    const methods = (calls: { method: string }[]) => calls.map((call) => call.method);

    it('renders the Cloud catalog when the host announces it, without touching the registries', async () => {
      const { wrapper, calls } = await explore();
      expect(methods(calls)).not.toContain(RPC.catalogItems);
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(names(wrapper)).toEqual(['Acme TTS']);
      expect(wrapper.find('.explore-toolbar__filters').exists()).toBe(false);
      expect(calls.filter(call => call.method === RPC.catalogItems).every(call => call.params?.kind === 'plugin')).toBe(true);
      expect(methods(calls)).toContain(RPC.catalogItems);
      expect(methods(calls)).not.toContain('registry.search');
      expect(methods(calls)).not.toContain('registry.list');
      expect(wrapper.text()).not.toMatch(/marketplace/i);
      expect(wrapper.find('.explore-toolbar').exists()).toBe(true);
    });

    it('falls back to the registry search when the host does not announce the catalog', async () => {
      const { wrapper, calls } = await explore(undefined, REGISTRY_HOST);
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(methods(calls)).toContain('registry.search');
      expect(methods(calls)).toContain('registry.list');
      expect(methods(calls)).not.toContain(RPC.catalogItems);
      expect(wrapper.find('.explore-toolbar').exists()).toBe(false);
      expect(wrapper.text()).toContain('Acme TTS');
    });

    it('keeps calling the catalog while discovery has not answered', async () => {
      const { wrapper, calls } = await explore(undefined, []);
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(methods(calls)).toContain(RPC.catalogItems);
      expect(methods(calls)).not.toContain('registry.search');
      expect(names(wrapper)).toEqual(['Acme TTS']);
      expect(wrapper.find('.explore-toolbar__filters').exists()).toBe(false);
      expect(calls.filter(call => call.method === RPC.catalogItems).every(call => call.params?.kind === 'plugin')).toBe(true);
    });

    it('shows a neutral retry state when the Cloud is unreachable, never the marketplace wording', async () => {
      const { wrapper } = await explore((method) => {
        if (method === RPC.catalogItems) throw new ControlCallError('unavailable', 'cloud down', { cloud: { code: 'temporaryFailure' } });
        return undefined;
      });
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(wrapper.text()).toContain(t('exploreCatalog.unavailable'));
      expect(wrapper.find('button[aria-label="Try again"]').exists()).toBe(true);
      expect(wrapper.text()).not.toMatch(/marketplace/i);
    });

    it('shows a plugin-only empty state even when the backend returns templates', async () => {
      const { wrapper } = await explore((method) => (method === RPC.catalogItems
        ? { items: [wire('template', FIXTURE_ID.tpl1, 'Gift alert')], nextCursor: null, dataState: 'noActiveGeneration' }
        : undefined));
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(names(wrapper)).toEqual([]);
      expect(wrapper.text()).toContain(t('exploreCatalog.nothingPublished'));
      expect(wrapper.text()).not.toContain('Published plugins, templates and profiles');
      expect(wrapper.text()).not.toContain('not available');
    });

    it('opens the remote details of a catalog plugin and reviews it through the signed package', async () => {
      const { wrapper, calls } = await explore();
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      await wrapper.get('.explore-list__row button.explore-list__open').trigger('click');
      await flushPromises();
      expect(calls.find((call) => call.method === 'registry.package')?.params).toEqual({ id: PLUGIN_ID.acmeTts, registry: 'official' });
      expect(wrapper.text()).toContain('Reads chat aloud.');
    });

    it('offers no manual install strip for an id or an https address', async () => {
      const { wrapper } = await explore();
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(wrapper.find('input[placeholder="Plugin id or https address"]').exists()).toBe(false);
      const legacy = await mountPage(undefined, undefined, undefined, REGISTRY_HOST);
      await openTab(legacy.wrapper, t(RPC.pluginsCatalogBrowse));
      expect(legacy.wrapper.find('input[placeholder="Plugin id or https address"]').exists()).toBe(false);
    });

    it('lists bundled plugins that are not installed above the catalog and does not repeat them', async () => {
      const { wrapper } = await explore((method) => (method === RPC.catalogItems
        ? { items: [wire('plugin', 'tts', 'Bundled TTS in Cloud'), wire('template', FIXTURE_ID.tpl1, 'Gift alert')], nextCursor: null }
        : undefined));
      await openTab(wrapper, t(RPC.pluginsCatalogBrowse));
      expect(wrapper.findAll('.plugin-catalog__row').map((item) => item.text())).toEqual([expect.stringContaining('TTS')]);
      expect(names(wrapper)).toEqual([]);
    });
  });

  it('keeps only the update policy in the plugin preferences: no registry can be added, enabled or disabled', async () => {
    const { wrapper } = await mountPage();
    await buttonByLabel(wrapper, t('syncChanges.more')).trigger('click');
    menuItem(t('plugins.catalog.preferences')).click();
    await flushPromises();
    const panel = wrapper.get('.plugin-preferences');
    expect(panel.find('#plugin-update-policy-channel').exists()).toBe(true);
    expect(panel.find('#plugin-update-policy-auto').exists()).toBe(true);
    expect(panel.find('#plugin-update-policy-interval').exists()).toBe(true);
    expect(panel.text()).toContain(t('packages.sources.allowSideload'));
    expect(panel.findAll('input[type="url"], input[type="text"][placeholder*="http"]')).toHaveLength(0);
    expect(panel.findAll('button').map((button) => button.attributes('aria-label') ?? button.text()))
      .not.toEqual(expect.arrayContaining([expect.stringMatching(/add|remove|enable|disable|source|registry/i)]));
    expect(panel.text()).not.toMatch(/registr|source/i);
  });

  it('restores the selected plugin from the view state', async () => {
    localStorage.setItem(STORAGE_KEY.viewState, JSON.stringify({ version: 1, plugins: { selected: 'deck', view: 'plugins' } }));
    const { wrapper } = await mountPage();
    expect(detailsTitle(wrapper)).toBe('DECK');
  });
});
