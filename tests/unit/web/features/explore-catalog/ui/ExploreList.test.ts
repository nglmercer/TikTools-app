import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { DEFAULT_CATALOG_FILTERS } from '@/entities/catalog/model/catalog-query.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import ExploreList from '../../../../../../src/web/features/explore-catalog/ui/ExploreList.vue';
import { FIXTURE_ID, PLUGIN_ID, RPC } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

const imports = vi.hoisted(() => ({ importTemplate: vi.fn(), importProfile: vi.fn() }));
vi.mock('@/entities/cloud-bundles/model/use-library-import.ts', () => ({ useLibraryImport: () => imports }));

const SHA = 'a'.repeat(64);
function wire(kind: string, id: string, title: string, extra: Record<string, unknown> = {}) {
  return {
    identity: { kind, id },
    presentation: { title, summary: `${title} summary`, publisher: 'acme', categories: [] },
    compatibility: { targets: [] },
    releaseRef: kind === 'plugin' ? { type: 'pluginRelease', version: '1.2.0' } : { type: 'documentRevision', revision: 3, sha256: SHA },
    publication: { yanked: false },
    ...extra,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

type Handler = (method: string, params: Record<string, unknown>) => unknown;

async function mountList(handler: Handler, installed: string[] = [], announce: string[] | null = [RPC.catalogItems]) {
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === RPC.rpcDiscover) return { methods: (announce ?? []).map((name) => ({ name })) };
    return handler(method, (params ?? {}) as Record<string, unknown>);
  });
  const stores = createStoreHarness(client);
  if (announce) await stores.use(() => useCapabilityStore()).refresh(FIXTURE_ID.testHost);
  const catalog = stores.use(() => useCatalogStore());
  const wrapper = mountWithPlugins(ExploreList, {
    props: { installedIds: new Set(installed) },
    attachTo: document.body,
    global: { plugins: stores.plugins },
  });
  await flushPromises();
  return { wrapper, calls, catalog };
}

const rowTitles = (wrapper: Awaited<ReturnType<typeof mountList>>['wrapper']) =>
  wrapper.findAll('.explore-list__row').map((row) => row.get('.explore-list__name').text());

describe('ExploreList', () => {
  it('renders plugins, templates and profiles as rows with their own action', async () => {
    const { wrapper, calls } = await mountList(() => ({
      items: [wire('plugin', PLUGIN_ID.acmeTts, 'Acme TTS'), wire('template', FIXTURE_ID.tpl1, t('overlays.kinds.gift.name')), wire('profile', FIXTURE_ID.prof1, 'Starter pack')],
      nextCursor: null,
    }));
    expect(calls.find((call) => call.method === RPC.catalogItems)?.params).toMatchObject({ limit: 25 });
    expect(rowTitles(wrapper)).toEqual(['Acme TTS', t('overlays.kinds.gift.name'), 'Starter pack']);
    const [plugin, template, profile] = wrapper.findAll('.explore-list__row');
    expect(plugin!.get('[role="img"]').attributes('aria-label')).toBe('Plugin');
    expect(template!.get('[role="img"]').attributes('aria-label')).toBe('Template');
    expect(profile!.get('[role="img"]').attributes('aria-label')).toBe('Profile');
    expect(plugin!.text()).toContain('1.2.0');
    expect(plugin!.find('button[aria-label="Install Acme TTS"]').exists()).toBe(true);
    expect(template!.find('button[aria-label="Import template Gift alert"]').exists()).toBe(true);
    expect(profile!.find('button[aria-label="Import profile Starter pack"]').exists()).toBe(true);
    // Only a plugin opens a details view; a template or a profile is imported.
    expect(plugin!.find('button.explore-list__open').exists()).toBe(true);
    expect(template!.find('button.explore-list__open').exists()).toBe(false);
  });

  it('opens a plugin and tells whether it is installed; the install icon starts the signed package review', async () => {
    const { wrapper, calls } = await mountList(
      (method) => (method === RPC.pluginsPackagesInspect ? { change: 'install', name: 'x' } : {
        items: [wire('plugin', PLUGIN_ID.acmeTts, 'Acme TTS'), wire('plugin', 'acme.have', 'Acme Have')],
        nextCursor: null,
      }),
      ['acme.have'],
    );
    const [fresh, have] = wrapper.findAll('.explore-list__row');
    expect(have!.find('button[aria-label^="Install"]').exists()).toBe(false);
    expect(have!.find('[role="img"][aria-label="Installed"]').exists()).toBe(true);
    await fresh!.get('button.explore-list__open').trigger('click');
    await have!.get('button.explore-list__open').trigger('click');
    expect(wrapper.emitted('openPlugin')).toEqual([[PLUGIN_ID.acmeTts, false], ['acme.have', true]]);
    await fresh!.get('button[aria-label="Install Acme TTS"]').trigger('click');
    await flushPromises();
    expect(calls.at(-1)).toEqual({ method: RPC.pluginsPackagesInspect, params: { spec: { kind: 'registry', id: PLUGIN_ID.acmeTts } } });
  });

  it('does not repeat plugins the page already lists itself', async () => {
    const { wrapper } = await mountList(() => ({ items: [wire('plugin', 'bundled.tts', t('plugins.catalog.bundled')), wire('template', 'bundled.tts', 'Same id template')], nextCursor: null }));
    expect(rowTitles(wrapper)).toEqual([t('plugins.catalog.bundled'), 'Same id template']);
    await wrapper.setProps({ hiddenIds: new Set(['bundled.tts']) });
    expect(rowTitles(wrapper)).toEqual(['Same id template']);
  });

  it('imports a template or a profile through the single library import', async () => {
    imports.importTemplate.mockResolvedValue(undefined);
    imports.importProfile.mockResolvedValue(undefined);
    const { wrapper } = await mountList(() => ({ items: [wire('template', FIXTURE_ID.tpl1, t('overlays.kinds.gift.name')), wire('profile', FIXTURE_ID.prof1, 'Starter pack')], nextCursor: null }));
    await wrapper.get('button[aria-label="Import template Gift alert"]').trigger('click');
    await flushPromises();
    expect(imports.importTemplate).toHaveBeenCalledWith({ id: FIXTURE_ID.tpl1, revision: 3 });
    expect(wrapper.get('[role="status"]').text()).toContain('Imported');
    await wrapper.get('button[aria-label="Import profile Starter pack"]').trigger('click');
    await flushPromises();
    expect(imports.importProfile).toHaveBeenCalledWith({ id: FIXTURE_ID.prof1, revision: 3 });
  });

  it('asks to sign in when the import needs an account', async () => {
    imports.importTemplate.mockRejectedValue(new ControlCallError('signed_out', 'not signed in'));
    const { wrapper } = await mountList(() => ({ items: [wire('template', FIXTURE_ID.tpl1, t('overlays.kinds.gift.name'))], nextCursor: null }));
    await wrapper.get('button[aria-label="Import template Gift alert"]').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain(t('exploreCatalog.signInToImport'));
  });

  describe('when the backend is not available', () => {
    it('shows a neutral state with a retry icon, never the marketplace wording', async () => {
      let healthy = false;
      const { wrapper } = await mountList(() => {
        if (!healthy) throw new ControlCallError('unavailable', 'cloud down', { cloud: { code: 'offline' } });
        return { items: [wire('plugin', PLUGIN_ID.acmeTts, 'Acme TTS')], nextCursor: null };
      });
      expect(wrapper.text()).toContain(t('exploreCatalog.unavailable'));
      expect(wrapper.text()).toContain('The PC is offline');
      expect(wrapper.text()).not.toMatch(/marketplace/i);
      expect(wrapper.find('.explore-list__row').exists()).toBe(false);
      healthy = true;
      await wrapper.get('button[aria-label="Try again"]').trigger('click');
      await flushPromises();
      expect(rowTitles(wrapper)).toEqual(['Acme TTS']);
      expect(wrapper.text()).not.toContain('not available');
    });

    it('offers to sign in when the Cloud refuses the session', async () => {
      const { wrapper } = await mountList(() => {
        throw new ControlCallError('unavailable', 'no session', { cloud: { code: 'unauthorized' } });
      });
      expect(wrapper.text()).toContain(t('cloudFailure.unauthorized'));
      expect(wrapper.find('button[aria-label="Sign in to Cloud"]').exists()).toBe(true);
    });
  });

  describe('when no plugin registry is active', () => {
    it('says nothing is published yet when there is nothing at all', async () => {
      const { wrapper } = await mountList(() => ({ items: [], nextCursor: null, dataState: 'noActiveGeneration' }));
      expect(wrapper.text()).toContain(t('exploreCatalog.nothingPublished'));
      expect(wrapper.text()).not.toContain('not available');
      expect(wrapper.find('button[aria-label="Try again"]').exists()).toBe(false);
    });

    it('still renders templates and profiles, with a neutral note for the plugins', async () => {
      const { wrapper } = await mountList(() => ({
        items: [wire('template', FIXTURE_ID.tpl1, t('overlays.kinds.gift.name'))], nextCursor: null, dataState: 'noActiveGeneration',
      }));
      expect(rowTitles(wrapper)).toEqual([t('overlays.kinds.gift.name')]);
      expect(wrapper.text()).toContain(t('exploreCatalog.noPlugins'));
      expect(wrapper.text()).not.toContain('not available');
    });

    it('a ready catalog with no results is a plain empty state', async () => {
      const { wrapper } = await mountList(() => ({ items: [], nextCursor: null, dataState: 'ready' }));
      expect(wrapper.text()).toContain(t('exploreCatalog.noResults'));
      expect(wrapper.text()).not.toContain(t('exploreCatalog.nothingPublished'));
    });
  });

  it('applies only the newest query: a slow older reply is dropped', async () => {
    const slow = deferred<unknown>();
    const { wrapper, catalog } = await mountList((_method, params) =>
      params.query === 'old' ? slow.promise : { items: [wire('plugin', 'new.one', 'New plugin')], nextCursor: null });
    const stale = catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'old' });
    await catalog.setFilters({ ...DEFAULT_CATALOG_FILTERS, text: 'new' });
    slow.resolve({ items: [wire('plugin', 'old.one', 'Old plugin')], nextCursor: 'stale', dataState: 'noActiveGeneration' });
    await stale;
    await flushPromises();
    expect(rowTitles(wrapper)).toEqual(['New plugin']);
    expect(wrapper.text()).not.toContain(t('exploreCatalog.noPlugins'));
    expect(wrapper.find('button[aria-label="Load more"]').exists()).toBe(false);
  });

  it('loads the next page on demand', async () => {
    const { wrapper, calls } = await mountList((_method, params) =>
      params.cursor ? { items: [wire('plugin', 'b.two', 'Two')], nextCursor: null } : { items: [wire('plugin', FIXTURE_ID.aOne, 'One')], nextCursor: 'c1' });
    await wrapper.get('button[aria-label="Load more"]').trigger('click');
    await flushPromises();
    expect(rowTitles(wrapper)).toEqual(['One', 'Two']);
    expect(calls.filter((call) => call.method === RPC.catalogItems).at(-1)?.params).toMatchObject({ cursor: 'c1' });
    expect(wrapper.find('button[aria-label="Load more"]').exists()).toBe(false);
  });
});
