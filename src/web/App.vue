<script setup lang="ts">
import { reactive } from 'vue';
import { AnalyticsView } from './views/analytics-view.vue';
import { BehaviorView } from './views/behavior-view.vue';
import { ConnectView } from './views/connect-view.vue';
import { FeedView } from './views/feed-view.vue';
import { PluginsView } from './views/plugins-view.vue';
import { PointsView } from './views/points-view.vue';
import { SettingsView } from './views/settings-view.vue';
import NavigationRail from './components/nav-rail.vue';
import TopNav from './components/top-nav.vue';
import PluginProgressNotification from './components/plugin-progress-notification.vue';
import DialogHost from './components/ui/dialog-host.vue';
import { useAppController } from './composables/useAppController.ts';

const app = reactive(useAppController());
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
      :on-reconnect="app.handleReconnect"
      :on-disconnect="app.handleDisconnect"
    />

    <div class="workspace-body">
      <NavigationRail
        :locale="app.locale"
        :active-tab="app.activeTab"
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
        :telemetry="app.telemetry"
        :events="app.events"
      />

      <BehaviorView
        v-else-if="app.activeTab === 'behavior'"
        :locale="app.locale"
        :gifts="app.giftCatalog"
        :viewers="app.leaderboard"
        :snapshot="app.behavior"
        :runs="app.behaviorRuns"
        :test-runs="app.behaviorTestRuns"
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
        :on-open-media-picker="app.openMediaPicker"
        :on-install-plugin="app.handleInstallPlugin"
        :plugin-install-state="app.pluginInstallState"
        :on-confirm-replace="app.handleConfirmPluginReplace"
        :on-cancel-replace="app.handleCancelPluginReplace"
      />

      <ConnectView
        v-else-if="app.activeTab === 'connect'"
        :locale="app.locale"
        :unique-id="app.uniqueId"
        :cookie="app.cookie"
        :status="app.status"
        :recents="app.recents"
        :error="app.error"
        :on-unique-id-change="app.setUniqueId"
        :on-cookie-change="app.setCookie"
        :on-connect="() => app.handleConnect()"
        :on-pick-live="app.handlePickLive"
        :on-select-recent="app.handleSelectRecent"
      />

      <SettingsView
        v-else-if="app.activeTab === 'settings'"
        :locale="app.locale"
        :theme="app.theme"
        :on-locale-change="app.setLocale"
        :on-theme-change="app.setTheme"
      />
    </div>
  </div>
</template>
