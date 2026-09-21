<script setup lang="ts">
import { computed, reactive, type ComputedRef } from 'vue';
import { parsePluginNavId } from '../automation/plugins/declarative.ts';
import type { PluginPageDescriptor, PluginStatus } from '../automation/behavior/types.ts';
import { i18nText } from './i18n.ts';
import { AnalyticsView } from './views/analytics-view.vue';
import { BehaviorView } from './views/behavior-view.vue';
import { ConnectionsView } from './views/connections-view.vue';
import { FeedView } from './views/feed-view.vue';
import { PluginPageView } from './views/plugin-page-view.vue';
import { PluginsView } from './views/plugins-view.vue';
import { PointsView } from './views/points-view.vue';
import { SettingsView } from './views/settings-view.vue';
import { WidgetsView } from './views/widgets-view.vue';
import NavigationRail from './components/nav-rail.vue';
import TopNav from './components/top-nav.vue';
import PluginProgressNotification from './components/plugin-progress-notification.vue';
import DialogHost from './components/ui/dialog-host.vue';
import { useAppController } from './composables/useAppController.ts';

const controller = useAppController();
const app = reactive(controller);

// Read through the raw controller refs (not the reactive proxy) so these
// stay shallow for the type checker.
const activePluginPage: ComputedRef<PluginPageDescriptor | undefined> = computed(() => {
  const parsed = parsePluginNavId(controller.activeTab.value);
  if (!parsed) return undefined;
  // Annotated locals keep the generic-inference chain shallow for vue-tsc.
  const pages: PluginPageDescriptor[] = controller.pluginPages.value;
  return pages.find(
    (page) => page.pluginId === parsed.pluginId && page.id === parsed.pageId,
  );
});
const activePluginName: ComputedRef<string> = computed(() => {
  const page = activePluginPage.value;
  if (!page) return '';
  const plugins: PluginStatus[] = controller.behavior.value.plugins;
  const plugin = plugins.find((entry) => entry.descriptor.id === page.pluginId);
  return plugin ? i18nText(controller.locale.value, plugin.descriptor.name) : page.pluginId;
});
const activePluginSupportsProvisioning: ComputedRef<boolean> = computed(() => {
  const page = activePluginPage.value;
  if (!page) return false;
  const plugins: PluginStatus[] = controller.behavior.value.plugins;
  const plugin = plugins.find((entry) => entry.descriptor.id === page.pluginId);
  return plugin?.descriptor.supportsTokenProvisioning ?? false;
});
const activePluginUi = computed(() => {
  const page = activePluginPage.value;
  if (!page) return undefined;
  return controller.pluginUis.value.find((entry) => entry.pluginId === page.pluginId);
});
const activePluginBackend = computed(() => {
  const page = activePluginPage.value;
  if (!page) return undefined;
  return controller.createPluginBackend(page.pluginId);
});
</script>
<template>
  <div class="app-shell">
    <DialogHost :locale="app.locale" />
    <PluginProgressNotification
      v-if="app.pluginProgress"
      :notification="app.pluginProgress"
      :on-dismiss="app.dismissPluginProgress"
    />
    <TopNav
      :locale="app.locale"
      :theme="app.theme"
      :status="app.status"
      :active-creator="app.activeCreator"
      :on-theme-toggle="app.handleThemeToggle"
      :on-locale-toggle="app.handleLocaleToggle"
      :on-connect="() => (app.uniqueId || app.activeCreator ? app.handleConnect() : app.setActiveTab('connect'))"
      :on-reconnect="app.handleReconnect"
      :on-disconnect="app.handleDisconnect"
    />

    <div class="workspace-body">
      <NavigationRail
        :locale="app.locale"
        :active-tab="app.activeTab"
        :plugin-pages="app.pluginPages"
        :on-tab-change="app.setActiveTab"
      />

      <FeedView
        v-if="app.activeTab === 'feed'"
        :locale="app.locale"
        :events="app.events"
        :leaderboard="app.leaderboard"
        :top-viewers="app.topViewers"
        :live-viewers="app.liveViewers"
        :filter="app.filter"
        :search-query="app.searchQuery"
        :auto-scroll="app.autoScroll"
        :unread-count="app.unreadCount"
        :on-filter-change="app.setFilter"
        :on-search-change="app.setSearchQuery"
        :on-toggle-auto-scroll="app.handleToggleAutoScroll"
        :on-clear-feed="app.resetEvents"
        :stream-container-ref="app.setStreamContainerRef"
      />

      <PointsView
        v-else-if="app.activeTab === 'points'"
        :locale="app.locale"
        :config="app.pointsConfig"
        :leaderboard="app.leaderboard"
        :status="app.status"
        :on-update-config="app.handleUpdatePointsConfig"
        :on-reset-points="app.handleResetPoints"
        :on-adjust-points="app.handleAdjustPoints"
      />

      <AnalyticsView
        v-else-if="app.activeTab === 'analytics'"
        :locale="app.locale"
        :creator="app.activeCreator"
        :summary="app.analyticsSummary"
        :on-request-range="app.handleGetAnalyticsRange"
      />

      <BehaviorView
        v-else-if="app.activeTab === 'behavior'"
        :locale="app.locale"
        :gifts="app.giftCatalog"
        :viewers="app.leaderboard"
        :snapshot="app.behavior"
        :runs="app.behaviorRuns"
        :test-runs="app.behaviorTestRuns"
        :hotkey-status="app.hotkeyStatus"
        :last-hotkey-event="app.lastHotkeyEvent"
        :error="app.behaviorError"
        :on-save-action="app.handleSaveAction"
        :on-delete-action="app.handleDeleteAction"
        :on-set-action-enabled="app.handleSetActionEnabled"
        :on-test-action="app.handleTestAction"
        :on-save-event="app.handleSaveEvent"
        :on-delete-event="app.handleDeleteEvent"
        :on-set-event-enabled="app.handleSetEventEnabled"
        :on-test-event="app.handleTestEvent"
        :on-open-plugins="app.openPlugins"
        :on-open-media-picker="app.openMediaPicker"
        :action-options="app.actionOptions"
        :action-option-errors="app.actionOptionErrors"
        :on-get-action-options="app.handleGetActionOptions"
      />

      <PluginsView
        v-else-if="app.activeTab === 'plugins'"
        :locale="app.locale"
        :plugins="app.behavior.plugins"
        :actions="app.behavior.actions"
        :action-types="app.behavior.actionTypes"
        :error="app.behaviorError"
        :on-set-installed="app.handleSetPluginInstalled"
        :on-uninstall="app.handleUninstallPlugin"
        :on-set-enabled="app.handleSetPluginEnabled"
        :settings="app.pluginSettings"
        :on-get-settings="app.handleGetPluginSettings"
        :on-save-settings="app.handleSavePluginSettings"
        :action-options="app.actionOptions"
        :on-get-action-options="app.handleGetActionOptions"
        :connections="app.pluginConnections"
        :on-test-connection="app.handleTestPluginConnection"
        :on-open-media-picker="app.openMediaPicker"
        :on-install-plugin="app.handleInstallPlugin"
        :plugin-install-state="app.pluginInstallState"
        :on-confirm-replace="app.handleConfirmPluginReplace"
        :on-cancel-replace="app.handleCancelPluginReplace"
        :processors="app.processors"
        :processor-test="app.processorTest"
        :on-get-processor-status="app.handleGetProcessorStatus"
        :on-test-processor="app.handleTestProcessor"
      />

      <ConnectionsView
        v-else-if="app.activeTab === 'connect'"
        :locale="app.locale"
        :unique-id="app.uniqueId"
        :cookie="app.cookie"
        :status="app.status"
        :recents="app.recents"
        :error="app.error"
        :plugins="app.behavior.plugins"
        :plugin-pages="app.pluginPages"
        :connections="app.pluginConnections"
        :plugin-settings="app.pluginSettings"
        :action-options="app.actionOptions"
        :provision-states="app.pluginProvision"
        :on-unique-id-change="app.setUniqueId"
        :on-cookie-change="app.setCookie"
        :on-connect="() => app.handleConnect()"
        :on-disconnect="app.handleDisconnect"
        :on-reconnect="app.handleReconnect"
        :on-pick-live="app.handlePickLive"
        :on-select-recent="app.handleSelectRecent"
        :on-test-connection="app.handleTestPluginConnection"
        :on-get-settings="app.handleGetPluginSettings"
        :on-save-settings="app.handleSavePluginSettings"
        :on-get-action-options="app.handleGetActionOptions"
        :on-open-media-picker="app.openMediaPicker"
        :on-provision-token="app.handleProvisionPluginToken"
        :on-open-plugins="app.openPlugins"
      />

      <WidgetsView
        v-else-if="app.activeTab === 'widgets'"
        :locale="app.locale"
        :plugins="app.behavior.plugins"
        :settings="app.pluginSettings"
        :on-get-settings="app.handleGetPluginSettings"
      />

      <SettingsView
        v-else-if="app.activeTab === 'settings'"
        :locale="app.locale"
        :theme="app.theme"
        :on-locale-change="app.setLocale"
        :on-theme-change="app.setTheme"
      />

      <PluginPageView
        v-else-if="activePluginPage"
        :key="activePluginPage.pluginId + ':' + activePluginPage.id"
        :locale="app.locale"
        :page="activePluginPage"
        :plugin-name="activePluginName"
        :settings-state="app.pluginSettings[activePluginPage.pluginId]"
        :connection="app.pluginConnections[activePluginPage.pluginId]"
        :action-options="app.actionOptions"
        :action-option-errors="app.actionOptionErrors"
        :action-option-selected="app.actionOptionSelected"
        :on-get-settings="app.handleGetPluginSettings"
        :on-save-settings="app.handleSavePluginSettings"
        :on-get-action-options="app.handleGetActionOptions"
        :on-test-connection="app.handleTestPluginConnection"
        :on-open-media-picker="app.openMediaPicker"
        :on-execute-action="(actionType, config) => app.executePluginAction(activePluginPage?.pluginId ?? '', actionType, config, true)"
        :supports-provisioning="activePluginSupportsProvisioning"
        :provision-state="app.pluginProvision[activePluginPage.pluginId]"
        :on-provision-token="app.handleProvisionPluginToken"
        :ui="activePluginUi"
        :backend="activePluginBackend?.backend"
        :subscribe-topic="activePluginBackend?.subscribeTopic"
      />
    </div>
  </div>
</template>
