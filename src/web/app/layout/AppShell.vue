<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, markRaw, watch, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { isConnectionOnlyPage, pluginNavId } from '../../../automation/plugins/declarative.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import type { AppTab, BuiltinAppTab } from '@/types.ts';
import AppMenuBar from '@/widgets/app-menu-bar/ui/AppMenuBar.vue';
import type { NavigationItem, NavigationSection } from '@/widgets/app-navigation/model/navigation-types.ts';
import AppNavigation from '@/widgets/app-navigation/ui/AppNavigation.vue';
import AppStatusBar from '@/widgets/app-status-bar/ui/AppStatusBar.vue';
import PluginProgressToast from '@/widgets/plugin-progress/ui/PluginProgressToast.vue';
import { useAppRuntime } from '../model/use-app-runtime.ts';
import { tabFromRoute, tabLocation } from '../router/tab-routes.ts';

useAppRuntime();

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const navigation = useNavigationStore();
const { activeTab } = storeToRefs(navigation);
const connection = useConnectionStore();
const { status, activeCreator, uniqueId, reconnectAttempt, reconnectDelayMs } = storeToRefs(connection);
const { liveViewers } = storeToRefs(useLiveFeedStore());
const plugins = usePluginsStore();
const { pluginProgress } = storeToRefs(plugins);
const { pluginPages } = storeToRefs(useAutomationStore());
const localeStore = useLocaleStore();
const { locale } = storeToRefs(localeStore);

document.documentElement.dataset.theme = 'dark';
document.documentElement.style.colorScheme = 'dark';

const builtinPages: ReadonlyArray<{ tab: BuiltinAppTab; label: string; icon: Component }> = [
  { tab: 'feed', label: 'navigation.feed', icon: IconChat },
  { tab: 'points', label: 'navigation.points', icon: IconCoins },
  { tab: 'analytics', label: 'navigation.analytics', icon: IconChart },
  { tab: 'connect', label: 'navigation.connections', icon: IconLink },
  { tab: 'behavior', label: 'navigation.automations', icon: IconFlow },
  { tab: 'plugins', label: 'navigation.plugins', icon: IconPuzzle },
  { tab: 'widgets', label: 'navigation.overlays', icon: IconLayers },
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
    if (tab) navigation.open(tab);
  },
  { immediate: true },
);

watch(activeTab, (tab) => {
  if (tab !== routeTab.value) void router.push(tabLocation(tab));
});

function navigate(tab: string): void {
  void router.push(tabLocation(tab as AppTab));
}

function connect(): void {
  if (uniqueId.value || activeCreator.value) connection.handleConnect();
  else navigate('connect');
}
</script>

<template>
  <div class="app-layout">
    <PluginProgressToast
      v-if="pluginProgress"
      :progress="pluginProgress"
      @dismiss="plugins.dismissPluginProgress()"
    />
    <AppMenuBar
      :status="status"
      :locale="locale"
      :pages="menuPages"
      @connect="connect"
      @disconnect="connection.handleDisconnect()"
      @reconnect="connection.handleReconnect()"
      @navigate="navigate"
      @change-locale="localeStore.setLocale"
    />
    <div class="app-layout__body">
      <AppNavigation
        class="app-layout__navigation"
        :sections="sections"
      />
      <main class="app-layout__content">
        <RouterView />
      </main>
    </div>
    <AppStatusBar
      :status="status"
      :creator="activeCreator"
      :viewers="liveViewers"
      :reconnect-attempt="reconnectAttempt"
      :reconnect-delay-ms="reconnectDelayMs"
      :max-reconnect-attempts="connection.maxReconnectAttempts"
      @connect="connect"
      @disconnect="connection.handleDisconnect()"
      @reconnect="connection.handleReconnect()"
    />
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: var(--color-bg-app);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.app-layout__body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.app-layout__navigation {
  flex: none;
  width: 208px;
  border-right: var(--border-width) solid var(--color-border-subtle);
}

.app-layout__content {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
</style>
