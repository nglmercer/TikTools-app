<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAppUpdaterStore } from '../model/updater-store.ts';
import type { UpdateChannel } from '../api/updater-types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';

const { t, locale } = useI18n();
const updater = useAppUpdaterStore();
const status = computed(() => updater.status);
const busy = computed(() => ['checking', 'downloading', 'verifying', 'installing'].includes(status.value?.state ?? ''));
const progress = computed(() => status.value?.totalBytes ? Math.min(100, Math.round(status.value.downloadedBytes * 100 / status.value.totalBytes)) : null);
const checked = computed(() => updater.preferences.lastChecked ? new Date(updater.preferences.lastChecked * 1000).toLocaleString(locale.value) : '');
function install(): void {
  window.ipc?.postMessage(JSON.stringify({ type: 'app-update-install', locale: locale.value }));
}
</script>

<template>
  <BaseGroupBox :title="t('appUpdater.title')">
    <div class="app-updater">
      <p v-if="status">
        {{ t('appUpdater.installed', { version: status.currentVersion }) }}
      </p>
      <p v-if="status?.availableVersion">
        {{ t('appUpdater.available', { version: status.availableVersion }) }}
      </p>
      <p v-if="status?.state === 'upToDate'">
        {{ t('appUpdater.upToDate') }}
      </p>
      <p v-if="checked">
        {{ t('appUpdater.lastChecked', { when: checked }) }}
      </p>
      <label for="app-update-channel">{{ t('appUpdater.channel') }}</label>
      <select
        id="app-update-channel"
        :value="updater.preferences.channel"
        :disabled="busy"
        @change="updater.setChannel(($event.target as HTMLSelectElement).value as UpdateChannel)"
      >
        <option value="stable">
          {{ t('appUpdater.stable') }}
        </option>
        <option value="beta">
          {{ t('appUpdater.beta') }}
        </option>
      </select>
      <BaseCheckbox
        :model-value="updater.preferences.automaticChecks"
        @update:model-value="updater.setPreference('automaticChecks', $event)"
      >
        {{ t('appUpdater.automaticChecks') }}
      </BaseCheckbox>
      <BaseCheckbox
        :model-value="updater.preferences.notifications"
        @update:model-value="updater.setPreference('notifications', $event)"
      >
        {{ t('appUpdater.notifications') }}
      </BaseCheckbox>
      <div class="app-updater__actions">
        <BaseButton
          :disabled="busy || !status?.checkSupported"
          @click="updater.check()"
        >
          {{ t('appUpdater.check') }}
        </BaseButton>
        <BaseButton
          v-if="status?.state === 'available' && status.installSupported"
          :disabled="busy"
          @click="updater.download()"
        >
          {{ t('appUpdater.download') }}
        </BaseButton>
        <BaseButton
          v-if="status?.state === 'readyToInstall' && status.installSupported"
          variant="primary"
          @click="install"
        >
          {{ t('appUpdater.install') }}
        </BaseButton>
      </div>
      <p v-if="status?.state === 'checking'">
        {{ t('appUpdater.checking') }}
      </p>
      <p v-if="status?.state === 'downloading'">
        {{ progress === null ? t('appUpdater.downloading') : t('appUpdater.progress', { percent: progress }) }}
      </p>
      <progress
        v-if="status?.state === 'downloading'"
        :value="progress ?? undefined"
        max="100"
      />
      <p v-if="status?.state === 'verifying'">
        {{ t('appUpdater.verifying') }}
      </p>
      <p v-if="status?.state === 'installing'">
        {{ t('appUpdater.closing') }}
      </p>
      <p
        v-if="status?.releaseNotes"
        class="app-updater__notes"
      >
        {{ status.releaseNotes }}
      </p>
      <p
        v-if="updater.error || status?.error"
        role="alert"
      >
        {{ updater.error || status?.error?.message }}
      </p>
      <p v-if="!status?.checkSupported || !status?.installSupported">
        {{ t('appUpdater.manual') }} <span class="app-updater__url">https://github.com/nglmercer/TikTools-app/releases</span>
      </p>
    </div>
  </BaseGroupBox>
</template>

<style scoped>
.app-updater { display: flex; flex-direction: column; gap: var(--space-3); }
.app-updater p { margin: 0; }
.app-updater select { max-width: 240px; }
.app-updater__actions { display: flex; gap: var(--space-3); }
.app-updater__notes { white-space: pre-wrap; overflow-wrap: anywhere; }
.app-updater__url { overflow-wrap: anywhere; user-select: text; }
</style>
