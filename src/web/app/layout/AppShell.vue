<script setup lang="ts">
import { useDiagnostics } from '../model/use-diagnostics.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { useDiagnosticNavigation } from '@/shared/diagnostics/navigation.ts';
import { storeToRefs } from 'pinia';
import { computed, defineAsyncComponent, markRaw, onBeforeUnmount, onMounted, ref, watch, type Component } from 'vue';

// The panel and its code load only when Diagnostics is opened; closing unmounts it but never clears the log.
const DiagnosticsPanel = defineAsyncComponent(() => import('@/shared/diagnostics/DiagnosticsPanel.vue'));
const diagnosticsOpen = computed(() => diagnosticView.open.value);
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
import { onPullToRefresh, onSwipe } from '@/shared/lib/touch-gestures.ts';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import BaseNotification from '@/shared/ui/notification/BaseNotification.vue';
import { useAppUpdaterStore } from '@/features/app-updater/model/updater-store.ts';
import type { AppTab, BuiltinAppTab } from '@/types.ts';
import AppMenuBar from '@/widgets/app-menu-bar/ui/AppMenuBar.vue';
import type { NavigationItem, NavigationSection } from '@/widgets/app-navigation/model/navigation-types.ts';
import AppNavigation from '@/widgets/app-navigation/ui/AppNavigation.vue';
import AppStatusBar from '@/widgets/app-status-bar/ui/AppStatusBar.vue';
import PluginProgressToast from '@/widgets/plugin-progress/ui/PluginProgressToast.vue';
import { useAppRuntime } from '../model/use-app-runtime.ts';
import { tabFromRoute, tabLocation } from '../router/tab-routes.ts';

useAppRuntime();
useDiagnostics();
const diagnosticNavigation = useDiagnosticNavigation();
const { unread } = diagnosticLog;

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const navigation = useNavigationStore();
const { activeTab, sidebarCollapsed } = storeToRefs(navigation);
const drawerOpen = ref(false);
const isNarrow = ref(false);
let narrowQuery: MediaQueryList | null = null;

function onNarrowChange(event: MediaQueryListEvent): void {
  isNarrow.value = event.matches;
  if (!event.matches) drawerOpen.value = false;
}

const sidebarExpanded = computed(() => (isNarrow.value ? drawerOpen.value : !sidebarCollapsed.value));
const sidebarRail = computed(() => !isNarrow.value && sidebarCollapsed.value);
const connection = useConnectionStore();
const { status, activeCreator, uniqueId, reconnectAttempt, reconnectDelayMs } = storeToRefs(connection);
const { liveViewers } = storeToRefs(useLiveFeedStore());
const plugins = usePluginsStore();
const { pluginProgress } = storeToRefs(plugins);
const { pluginPages } = storeToRefs(useAutomationStore());
const localeStore = useLocaleStore();
const appUpdater = useAppUpdaterStore();
const dismissedUpdate = ref('');
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

function toggleSidebar(): void {
  if (isNarrow.value) drawerOpen.value = !drawerOpen.value;
  else navigation.toggleSidebar();
}

function closeDrawer(): void {
  drawerOpen.value = false;
}

/* ---- Touch gestures (mobile) ---- */
const contentRef = ref<HTMLElement | null>(null);
const pullDistance = ref(0);
const pullArmed = ref(false);
const gestureDisposers: Array<() => void> = [];

function reloadApp(): void {
  pullDistance.value = 0;
  window.location.reload();
}

function installGestures(): void {
  // Swipe in from the left edge opens the drawer on narrow layouts.
  gestureDisposers.push(onSwipe(window, {
    edge: 24,
    enabled: () => isNarrow.value && !drawerOpen.value,
    onSwipe: (direction) => { if (direction === 'right') drawerOpen.value = true; },
  }));
  // Swiping left on the open drawer (or its scrim) closes it.
  gestureDisposers.push(onSwipe(window, {
    enabled: () => isNarrow.value && drawerOpen.value,
    onSwipe: (direction) => { if (direction === 'left') drawerOpen.value = false; },
  }));
  if (contentRef.value) {
    gestureDisposers.push(onPullToRefresh(contentRef.value, {
      onPull: (distance, armed) => {
        pullDistance.value = distance;
        pullArmed.value = armed;
      },
      onRefresh: reloadApp,
    }));
  }
}

function onShellKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && isNarrow.value && drawerOpen.value && !event.defaultPrevented) {
    drawerOpen.value = false;
  }
}

onMounted(() => {
  if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
    narrowQuery = window.matchMedia('(width < 900px)');
    isNarrow.value = narrowQuery.matches;
    if (typeof narrowQuery.addEventListener === 'function') narrowQuery.addEventListener('change', onNarrowChange);
    else narrowQuery.addListener(onNarrowChange as (event: MediaQueryListEvent) => void);
  }
  window.addEventListener('keydown', onShellKeydown);
  installGestures();
});

onBeforeUnmount(() => {
  if (narrowQuery) {
    if (typeof narrowQuery.removeEventListener === 'function') narrowQuery.removeEventListener('change', onNarrowChange);
    else narrowQuery.removeListener(onNarrowChange as (event: MediaQueryListEvent) => void);
    narrowQuery = null;
  }
  window.removeEventListener('keydown', onShellKeydown);
  for (const dispose of gestureDisposers.splice(0)) dispose();
});

function connect(): void {
  const target = uniqueId.value.trim().replace(/^@/, '');
  if (target) {
    connection.handleConnect(target);
  } else {
    connection.handleConnect();
  }
}


</script>

<template>
  <div
    class="app-layout"
    :class="{ 'app-layout--automation': route.path === '/automations' }"
  >
    <PluginProgressToast
      v-if="pluginProgress"
      :progress="pluginProgress"
      @dismiss="plugins.dismissPluginProgress()"
    />
    <BaseNotification
      v-if="appUpdater.preferencesLoaded && appUpdater.preferences.notifications && appUpdater.status?.channel === appUpdater.preferences.channel && appUpdater.status?.state === 'available' && appUpdater.status.availableVersion && dismissedUpdate !== appUpdater.status.availableVersion"
      class="app-layout__notice"
      :message="t('appUpdater.available', { version: appUpdater.status.availableVersion })"
      :action-label="t('appUpdater.title')"
      @action="router.push(tabLocation('settings'))"
      @dismiss="dismissedUpdate = appUpdater.status?.availableVersion ?? ''"
    />
    <AppMenuBar
      :status="status"
      :locale="locale"
      :pages="menuPages"
      :sidebar-expanded="sidebarExpanded"
      @connect="connect"
      @disconnect="connection.handleDisconnect()"
      @reconnect="connection.handleReconnect()"
      @navigate="navigate"
      @change-locale="localeStore.setLocale"
      @toggle-sidebar="toggleSidebar"
    />
    <div class="app-layout__body">
      <button
        v-if="drawerOpen"
        class="app-layout__scrim"
        type="button"
        :aria-label="t('navigation.toggleSidebar')"
        @click="closeDrawer"
      />
      <AppNavigation
        id="app-sidebar"
        class="app-layout__navigation"
        :class="{ 'app-layout__navigation--collapsed': sidebarRail, 'app-layout__navigation--open': drawerOpen }"
        :sections="sections"
        :collapsed="sidebarRail"
      />
      <main
        ref="contentRef"
        class="app-layout__content"
      >
        <div
          v-if="pullDistance > 0"
          class="app-layout__pull"
          :class="{ 'app-layout__pull--armed': pullArmed }"
          :style="{ transform: `translate(-50%, ${pullDistance - 40}px)` }"
          role="status"
        >
          <IconRefresh
            class="app-layout__pull-icon"
            :style="{ transform: `rotate(${pullDistance * 3}deg)` }"
          />
          <span>{{ t(pullArmed ? 'navigation.pull.release' : 'navigation.pull.hint') }}</span>
        </div>
        <RouterView />
      </main>
    </div>

    <DiagnosticsPanel v-if="diagnosticsOpen" />
    <AppStatusBar
      :status="status"
      :hide-connection="routeTab === 'connect'"
      :creator="activeCreator"
      :viewers="liveViewers"
      :reconnect-attempt="reconnectAttempt"
      :reconnect-delay-ms="reconnectDelayMs"
      :max-reconnect-attempts="connection.maxReconnectAttempts"
      @connect="connect"
      @disconnect="connection.handleDisconnect()"
      @reconnect="connection.handleReconnect()"
    >
      <button
        v-if="route.path !== '/debug'"
        class="diagnostic-launcher"
        @click="diagnosticNavigation.open()"
      >
        {{ t('diagnostics.title') }}<span
          v-if="unread"
          aria-live="polite"
        > · {{ unread }}</span>
      </button>
    </AppStatusBar>
  </div>
</template>

<style scoped>
.diagnostic-launcher { flex:none; min-height:var(--control-height-md); padding:var(--space-2) var(--space-5); border-radius:var(--radius-md); color:var(--color-text); background:var(--color-bg-panel); border:var(--border-width) solid var(--color-border); cursor:pointer; }
.diagnostic-launcher:focus-visible { outline:2px solid var(--color-border-focus); outline-offset:2px; }
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

.app-layout__notice {
  position: fixed;
  z-index: 100;
  top: 42px;
  right: var(--space-4);
  width: 440px;
  max-width: min(440px, calc(100vw - 32px));
}

/* Pull-to-refresh indicator: slides down from the top of the content. */
.app-layout__pull {
  position: absolute;
  z-index: 30;
  top: 0;
  left: 50%;
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  height: 32px;
  padding: 0 var(--space-5);
  border: var(--border-width) solid var(--color-border);
  border-radius: 999px;
  background: var(--color-bg-overlay);
  box-shadow: var(--shadow-popup);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  white-space: nowrap;
  pointer-events: none;
}

.app-layout__pull--armed {
  border-color: var(--color-border-selected);
  color: var(--color-text);
}

.app-layout__pull--armed .app-layout__pull-icon {
  color: var(--color-accent-hover);
}

.app-layout__navigation {
  flex: none;
  width: 208px;
  border-right: var(--border-width) solid var(--color-border-subtle);
}

.app-layout__navigation--collapsed {
  width: 56px;
}

.app-layout__scrim {
  position: fixed;
  inset: 0;
  z-index: 40;
  width: 100%;
  height: 100%;
  padding: 0;
  border: 0;
  background: rgb(0 0 0 / 45%);
  cursor: pointer;
}

.app-layout__content {
  position: relative;
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

@media (width < 1024px) {
  .app-layout__navigation {
    width: 180px;
  }

  .app-layout__navigation--collapsed {
    width: 56px;
  }
}

@media (width < 900px) {
  .app-layout {
    height: 100dvh;
  }

  .app-layout__navigation {
    position: fixed;
    inset: var(--chrome-height) auto 0 0;
    z-index: 50;
    width: min(280px, 84vw);
    height: auto;
    border-right: var(--border-width) solid var(--color-border-subtle);
    box-shadow: var(--shadow-popup);
    transform: translateX(-105%);
    transition: transform 160ms ease;
  }

  .app-layout__navigation--open {
    transform: none;
  }

  .app-layout__navigation--collapsed {
    width: min(280px, 84vw);
  }

  .app-layout__content {
    overflow: auto;
  }
}

@media (prefers-reduced-motion: reduce) {
  .app-layout__navigation {
    transition: none;
  }
}
</style>
