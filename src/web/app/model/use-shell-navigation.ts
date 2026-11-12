import { computed, markRaw, watch, type Component, type Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { isConnectionOnlyPage, pluginNavId } from '../../../automation/plugins/declarative.ts';
import type { PluginPageDescriptor } from '../../../automation/behavior/types.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import type { AppTab, BuiltinAppTab } from '@/types.ts';
import type { NavigationItem, NavigationSection } from '@/widgets/app-navigation/model/navigation-types.ts';
import { tabFromRoute, tabLocation } from '../router/tab-routes.ts';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';

export function useShellNavigation(
  pluginPages: Readonly<Ref<PluginPageDescriptor[]>>,
  navigation: { activeTab: Readonly<Ref<AppTab>>; open: (tab: AppTab) => void },
  drawerOpen: Ref<boolean>,
) {
  const { t } = useI18n();
  const route = useRoute();
  const router = useRouter();
  const { activeTab } = navigation;
  const builtinPages: ReadonlyArray<{ tab: BuiltinAppTab; label: string; icon: Component }> = [
    { tab: 'feed', label: 'navigation.feed', icon: IconChat },
    { tab: 'points', label: 'navigation.points', icon: IconCoins },
    { tab: 'analytics', label: 'navigation.analytics', icon: IconChart },
    { tab: 'connect', label: 'navigation.connections', icon: IconLink },
    { tab: 'behavior', label: 'navigation.automations', icon: IconFlow },
    { tab: 'plugins', label: 'navigation.plugins', icon: IconPuzzle },
    { tab: 'widgets', label: 'navigation.overlays', icon: IconLayers },
    // Rankings & competitions is still in development: the route stays
    // reachable (#/rankings) and the ranking engine feeds the Live feed, but
    // the page is not listed in navigation yet.
    // { tab: 'rankings', label: 'navigation.rankings', icon: IconChart },
    { tab: 'settings', label: 'navigation.settings', icon: IconGear },
  ];

  const builtinItems = computed<NavigationItem[]>(() =>
    builtinPages.map((page) => ({
      id: page.tab,
      label: t(page.label),
      icon: markRaw(page.icon),
      to: tabLocation(page.tab),
    })),
  );

  const pluginItems = computed<NavigationItem[]>(() =>
    pluginPages.value
      .filter((page) => !isConnectionOnlyPage(page))
      .map((page) => {
        const tab = pluginNavId(page.pluginId, page.id);
        return { id: tab, label: localizedText(page.title), icon: markRaw(IconPuzzle), to: tabLocation(tab) };
      }),
  );

  const sections = computed<NavigationSection[]>(() => [
    { id: 'workspace', label: t('navigation.sections.workspace'), items: builtinItems.value },
    ...(pluginItems.value.length ? [{ id: 'plugins', label: t('navigation.sections.plugins'), items: pluginItems.value }] : []),
  ]);

  const menuPages = computed(() => sections.value.flatMap((section) => section.items));
  const routeTab = computed(() => tabFromRoute(route));

  watch(
    routeTab,
    (tab) => {
      drawerOpen.value = false;
      if (tab) navigation.open(tab);
    },
    { immediate: true },
  );

  watch(activeTab, (tab) => {
    if (tab !== routeTab.value) void router.push(tabLocation(tab));
  });

  function navigate(tab: string): void {
    drawerOpen.value = false;
    void router.push(tabLocation(tab as AppTab));
  }

  return { route, router, routeTab, sections, menuPages, navigate };
}
