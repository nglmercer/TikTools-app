<script setup lang="ts">
import { useDiagnostics } from '../model/use-diagnostics.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { useDiagnosticNavigation } from '@/shared/diagnostics/navigation.ts';
import { storeToRefs } from 'pinia';
import { computed, defineAsyncComponent, useTemplateRef } from 'vue';

// The panel and its code load only when Diagnostics is opened; closing unmounts it but never clears the log.
const DiagnosticsPanel = defineAsyncComponent(() => import('@/shared/diagnostics/DiagnosticsPanel.vue'));
const diagnosticsOpen = computed(() => diagnosticView.open.value);
import { useI18n } from 'vue-i18n';
import type { Locale } from '@/shared/i18n/i18n.ts';
import { applyAppTheme } from '@/shared/lib/app-theme.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { loadResolutionPreference } from '@/features/review-shared/model/resolution-preference.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import OwnerLibraryReview from '@/features/recover-owner-library/ui/OwnerLibraryReview.vue';
import AppMenuBar from '@/widgets/app-menu-bar/ui/AppMenuBar.vue';
import AppNavigation from '@/widgets/app-navigation/ui/AppNavigation.vue';
import AppStatusBar from '@/widgets/app-status-bar/ui/AppStatusBar.vue';
import { mergePluginNavigationPages } from '../../../automation/plugins/declarative.ts';
import { useAppRuntime } from '../model/use-app-runtime.ts';
import { tabLocation } from '../router/tab-routes.ts';

import { useShellInteractions } from '../model/use-shell-interactions.ts';
import { useShellNavigation } from '../model/use-shell-navigation.ts';
import AppNotifications from './AppNotifications.vue';

useAppRuntime();
useDiagnostics();
void loadResolutionPreference(useControlClient());
const diagnosticNavigation = useDiagnosticNavigation();
const menuBar = useTemplateRef<InstanceType<typeof AppMenuBar>>('menuBar');
const { unread } = diagnosticLog;

const { t } = useI18n();
const navigation = useNavigationStore();
const { activeTab, sidebarCollapsed } = storeToRefs(navigation);
const interactions = useShellInteractions(sidebarCollapsed, () => navigation.toggleSidebar());
const { drawerOpen, sidebarExpanded, sidebarRail, pullDistance, pullArmed, toggleSidebar, closeDrawer } = interactions;
const connection = useConnectionStore();
const { status, activeCreator, uniqueId, reconnectAttempt, reconnectDelayMs } = storeToRefs(connection);
const { liveViewers } = storeToRefs(useLiveFeedStore());
const { pluginPages, pluginUis } = storeToRefs(useAutomationStore());
const localeStore = useLocaleStore();
const { locale } = storeToRefs(localeStore);

// The demo sets the theme from its host (or the OS) before mounting. The
// desktop and local-web apps keep their historical dark default.
if (!document.documentElement.dataset.theme) applyAppTheme('dark');

const pluginNavigationPages = computed(() => mergePluginNavigationPages(pluginPages.value, pluginUis.value));
const { route, router, routeTab, sections, menuPages, navigate } = useShellNavigation(pluginNavigationPages, {
  activeTab, open: navigation.open,
}, drawerOpen);
// vite-plugin-vue infers each page component's name from its filename, so
// keep-alive can match them without a manual `name` option.
const cachedTabPages = [
  'FeedPage',
  'PointsPage',
  'AnalyticsPage',
  'ConnectionsPage',
  'AutomationsPage',
  'PluginsPage',
  'OverlaysPage',
  'RankingsPage',
  'SettingsPage',
];

function connect(): void {
  const target = uniqueId.value.trim().replace(/^@/, '');
  if (target) {
    connection.handleConnect(target);
  } else {
    connection.handleConnect();
  }
}

// The desktop shell renders the menu bar natively in the GTK header bar and
// reports activations back as pushes; dispatch them through the exact same
// handlers the in-page menu uses. The in-page bar stays mounted (hidden) so
// the menu model keeps shipping, but only one of the two is ever visible.
const desktopShell = typeof window !== 'undefined' && !!window.ipc;
if (desktopShell) {
  useControlClient().onPush('shell-menu-action', (message) => {
    if (message.type !== 'shell-menu-action') return;
    const id = message.id;
    const separator = id.indexOf(':');
    const kind = separator < 0 ? id : id.slice(0, separator);
    const value = separator < 0 ? undefined : id.slice(separator + 1);
    if (kind === 'connect') connect();
    else if (kind === 'reconnect') connection.handleReconnect();
    else if (kind === 'disconnect') connection.handleDisconnect();
    else if (kind === 'navigate' && value) navigate(value);
    else if (kind === 'locale' && value) localeStore.setLocale(value as Locale);
    else if (kind === 'toggle-sidebar') toggleSidebar();
    else if (kind === 'help' && value === 'guides') menuBar.value?.openGuides();
  });
}

</script>

<template>
  <div
    class="app-layout"
    :class="{ 'app-layout--automation': route.path === '/automations', 'app-layout--native-chrome': desktopShell }"
  >
    <AppNotifications @open-settings="router.push(tabLocation('settings'))" />
    <AppMenuBar
      ref="menuBar"
      v-show="!desktopShell"
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
        :ref="interactions.contentRef"
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
        <RouterView v-slot="{ Component }">
          <!-- Builtin tab pages stay mounted once visited: pinia stores are
               global, so remounting only rebuilt the DOM tree from scratch on
               every tab switch. Plugin/debug pages are excluded — a single
               component type serves many parameterised routes, and detached
               live views should not keep consuming a plugin's render loop. -->
          <KeepAlive :include="cachedTabPages">
            <component :is="Component" />
          </KeepAlive>
        </RouterView>
      </main>
    </div>

    <!-- The one review of Owner library changes (Cloud imports and profile changes), whatever page started them. -->
    <OwnerLibraryReview />
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

<style src="./app-shell.css"></style>
