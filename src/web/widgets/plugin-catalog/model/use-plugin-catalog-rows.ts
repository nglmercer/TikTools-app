import { computed, type Ref } from 'vue';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { catalogPlugins, type PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import type { UpdateInfo } from '@/entities/plugin-package/api/package-types.ts';
import { remoteSelection } from '@/entities/plugin-package/lib/package-format.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import type { CatalogRow, PluginCatalogProps } from './catalog-types.ts';

export function usePluginCatalogRows(props: Readonly<PluginCatalogProps>, tab: Readonly<Ref<PluginsPanelTab>>, query: Readonly<Ref<string>>) {
  const catalogStore = useCatalogStore();
  const installedCount = computed(() => props.plugins.filter((plugin) => plugin.installed).length);
  const pendingUpdates = computed(() => props.updates.filter((entry) => entry.latest && entry.latestStatus === 'approved' && !entry.pinned));
  function updateName(update: UpdateInfo): string {
    const plugin = props.plugins.find((entry) => entry.descriptor.id === update.id);
    return plugin ? localizedText(plugin.descriptor.name) : update.id;
  }

  /** Explore shows the Cloud catalog only when the host serves it. */
  const exploring = computed(() => props.exploreCatalog === true && tab.value === 'browse');
  const installedIds = computed(() => new Set(props.plugins.filter((plugin) => plugin.installed).map((plugin) => plugin.descriptor.id)));
  /** Bundled plugins already listed above the catalog: the catalog does not repeat them. */
  const listedLocally = computed(() => new Set(props.plugins.filter((plugin) => !plugin.installed).map((plugin) => plugin.descriptor.id)));

  const rows = computed<CatalogRow[]>(() => {
    if (tab.value === 'updates') {
      const words = query.value.trim().toLowerCase();
      return props.updates
        .filter((update) => !words || `${update.id} ${updateName(update)}`.toLowerCase().includes(words))
        .map((update) => ({
          key: update.id,
          kind: 'update' as const,
          update,
          plugin: props.plugins.find((plugin) => plugin.descriptor.id === update.id),
        }));
    }
    if (exploring.value) {
      // Bundled plugins that are not installed yet stay here; the Cloud catalog lists the rest below them.
      const { type, view, text } = catalogStore.filters;
      if (view === 'mine' || (type !== 'all' && type !== 'plugin')) return [];
      return catalogPlugins(props.plugins, 'browse', props.actionTypes, text, localizedText).map((plugin) => ({
        key: plugin.descriptor.id,
        kind: 'local' as const,
        plugin,
      }));
    }
    const local = catalogPlugins(props.plugins, tab.value === 'installed' ? 'installed' : 'browse', props.actionTypes, query.value, localizedText).map(
      (plugin) => ({ key: plugin.descriptor.id, kind: 'local' as const, plugin }),
    );
    if (tab.value === 'installed') return local;
    const known = new Set(props.plugins.map((plugin) => plugin.descriptor.id));
    const remote = props.remote
      .filter((entry) => !known.has(entry.id))
      .map((entry) => ({ key: remoteSelection(entry.registry, entry.id), kind: 'remote' as const, entry }));
    return [...local, ...remote];
  });


  return { rows, installedCount, pendingUpdates, exploring, installedIds, listedLocally };
}
