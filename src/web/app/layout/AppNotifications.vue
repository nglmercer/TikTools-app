<script setup lang="ts">
import { ref } from 'vue';
import { storeToRefs } from 'pinia';
import { useI18n } from 'vue-i18n';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useAppUpdaterStore } from '@/features/app-updater/model/updater-store.ts';
import BaseNotification from '@/shared/ui/notification/BaseNotification.vue';
import PluginProgressToast from '@/widgets/plugin-progress/ui/PluginProgressToast.vue';
const emit = defineEmits<{ openSettings: [] }>();
const { t } = useI18n();
const plugins = usePluginsStore();
const { pluginProgress } = storeToRefs(plugins);
const appUpdater = useAppUpdaterStore();
const dismissedUpdate = ref('');
</script>

<template>
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
    @action="emit('openSettings')"
    @dismiss="dismissedUpdate = appUpdater.status?.availableVersion ?? ''"
  />
</template>
