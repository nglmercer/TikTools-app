<script setup lang="ts">
import { useDiagnostics } from '../model/use-diagnostics.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { useDiagnosticNavigation } from '@/shared/diagnostics/navigation.ts';
import { storeToRefs } from 'pinia';
import { computed, defineAsyncComponent } from 'vue';

// The panel and its code load only when Diagnostics is opened; closing unmounts it but never clears the log.
const DiagnosticsPanel = defineAsyncComponent(() => import('@/shared/diagnostics/DiagnosticsPanel.vue'));
const diagnosticsOpen = computed(() => diagnosticView.open.value);
import { useI18n } from 'vue-i18n';
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
import { useAppRuntime } from '../model/use-app-runtime.ts';
import { tabLocation } from '../router/tab-routes.ts';

import { useShellInteractions } from '../model/use-shell-interactions.ts';
import { useShellNavigation } from '../model/use-shell-navigation.ts';
import AppNotifications from './AppNotifications.vue';

useAppRuntime();
useDiagnostics();
const diagnosticNavigation = useDiagnosticNavigation();
const { unread } = diagnosticLog;

const { t } = useI18n();
const navigation = useNavigationStore();
const { activeTab, sidebarCollapsed } = storeToRefs(navigation);
const interactions = useShellInteractions(sidebarCollapsed, () => navigation.toggleSidebar());
const { drawerOpen, sidebarExpanded, sidebarRail, pullDistance, pullArmed, toggleSidebar, closeDrawer } = interactions;
const connection = useConnectionStore();
const { status, activeCreator, uniqueId, reconnectAttempt, reconnectDelayMs } = storeToRefs(connection);
const { liveViewers } = storeToRefs(useLiveFeedStore());
const { pluginPages } = storeToRefs(useAutomationStore());
const localeStore = useLocaleStore();
const { locale } = storeToRefs(localeStore);

document.documentElement.dataset.theme = 'dark';
document.documentElement.style.colorScheme = 'dark';

const { route, router, routeTab, sections, menuPages, navigate } = useShellNavigation(pluginPages, {
  activeTab, open: navigation.open,
}, drawerOpen);

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
    <AppNotifications @open-settings="router.push(tabLocation('settings'))" />
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
        <RouterView />
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
