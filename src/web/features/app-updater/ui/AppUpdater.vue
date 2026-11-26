<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAppUpdaterStore } from '../model/updater-store.ts';
import type { UpdateChannel } from '../api/updater-types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconHardDrive from '@/shared/ui/icon/icons/IconHardDrive.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';

const { t, locale } = useI18n();
const updater = useAppUpdaterStore();
const status = computed(() => updater.status);
const unavailable = computed(() => status.value?.checkSupported === false || /unavailable in this host/i.test(updater.error || status.value?.error?.message || ''));
const matchingChannel = computed(() => updater.preferencesLoaded && status.value?.channel === updater.preferences.channel);
const busy = computed(() => ['checking', 'downloading', 'verifying', 'installing'].includes(status.value?.state ?? ''));
const progress = computed(() => status.value?.totalBytes ? Math.min(100, Math.round(status.value.downloadedBytes * 100 / status.value.totalBytes)) : null);
const checked = computed(() => updater.preferences.lastChecked ? new Date(updater.preferences.lastChecked * 1000).toLocaleString(locale.value) : '');
const channels = computed(() => [
  { value: 'stable' as UpdateChannel, label: t('appUpdater.stable') },
  { value: 'beta' as UpdateChannel, label: t('appUpdater.beta') },
]);
function setChannel(value: UpdateChannel | undefined): void {
  if (value && value !== updater.preferences.channel) void updater.setChannel(value);
}
function install(): void {
  window.ipc?.postMessage(JSON.stringify({ type: 'app-update-install', locale: locale.value }));
}
const checking = computed(() => status.value?.state === 'checking');
const downloading = computed(() => status.value?.state === 'downloading');
const downloadFailed = computed(() => status.value?.state === 'failed' && status.value?.error?.code === 'download_failed');
// The install handoff is fire-and-forget by design (the process exits on
// success), so the button must latch its own pending feedback: the user
// otherwise sees a live-looking button while the native confirmation is up.
const installRequested = ref(false);
function requestInstall(): void {
  installRequested.value = true;
  install();
}
</script>

<template>
  <BaseGroupBox :title="t('appUpdater.title')">
    <div class="app-updater">
      <div
        v-if="status || checked"
        class="app-updater__facts"
      >
        <p v-if="status">
          {{ t('appUpdater.installed', { version: status.currentVersion }) }}
        </p>
        <p
          v-if="status?.availableVersion && matchingChannel"
          class="app-updater__highlight"
        >
          {{ t('appUpdater.available', { version: status.availableVersion }) }}
        </p>
        <p v-if="status?.state === 'upToDate' && matchingChannel">
          {{ t('appUpdater.upToDate') }}
        </p>
        <p
          v-if="checked"
          class="app-updater__muted"
        >
          {{ t('appUpdater.lastChecked', { when: checked }) }}
        </p>
      </div>
      <div class="app-updater__settings">
        <SettingsRow
          for="app-update-channel"
          :label="t('appUpdater.channel')"
        >
          <BaseComboBox
            id="app-update-channel"
            class="app-updater__channel"
            :model-value="updater.preferences.channel"
            :options="channels"
            :disabled="busy || !updater.preferencesLoaded"
            @update:model-value="setChannel"
          />
        </SettingsRow>
        <SettingsRow
          for="app-update-automatic"
          :label="t('appUpdater.automaticChecks')"
          :description="t('formsDesign.automaticChecksHint')"
          toggle
        >
          <BaseSwitch
            id="app-update-automatic"
            :aria-label="t('appUpdater.automaticChecks')"
            :model-value="updater.preferences.automaticChecks"
            :disabled="!updater.preferencesLoaded"
            @update:model-value="updater.setPreference('automaticChecks', $event)"
          />
        </SettingsRow>
        <SettingsRow
          for="app-update-notifications"
          :label="t('appUpdater.notifications')"
          :description="t('formsDesign.notificationsHint')"
          toggle
        >
          <BaseSwitch
            id="app-update-notifications"
            :aria-label="t('appUpdater.notifications')"
            :model-value="updater.preferences.notifications"
            :disabled="!updater.preferencesLoaded"
            @update:model-value="updater.setPreference('notifications', $event)"
          />
        </SettingsRow>
      </div>
      <div class="app-updater__actions">
        <BaseButton
          :disabled="busy || !updater.preferencesLoaded || !status?.checkSupported"
          :aria-label="t('appUpdater.check')"
          @click="updater.check()"
        >
          <template #icon>
            <IconRefresh :class="{ 'app-updater__spin': checking }" />
          </template>
          <span class="app-updater__label">{{ checking ? t('appUpdater.checking') : t('appUpdater.check') }}</span>
        </BaseButton>
        <BaseButton
          v-if="(status?.state === 'available' || downloadFailed) && status?.installSupported && matchingChannel"
          :variant="downloadFailed ? 'danger' : 'default'"
          :disabled="busy"
          :aria-label="t(downloadFailed ? 'appUpdater.retryDownload' : 'appUpdater.download')"
          @click="updater.download()"
        >
          <template #icon>
            <IconWarning v-if="downloadFailed" />
            <IconDownload v-else />
          </template>
          <span class="app-updater__label">{{ t(downloadFailed ? 'appUpdater.retryDownload' : 'appUpdater.download') }}</span>
        </BaseButton>
        <BaseButton
          v-if="downloading"
          disabled
          :aria-label="t('appUpdater.downloading')"
        >
          <template #icon>
            <IconDownload class="app-updater__pulse" />
          </template>
          <span class="app-updater__label">{{ progress === null ? t('appUpdater.downloading') : t('appUpdater.progress', { percent: progress }) }}</span>
        </BaseButton>
        <BaseButton
          v-if="status?.state === 'readyToInstall' && status.installSupported && matchingChannel"
          variant="primary"
          :disabled="installRequested"
          :aria-label="t('appUpdater.install')"
          @click="requestInstall"
        >
          <template #icon>
            <IconHardDrive :class="{ 'app-updater__pulse': installRequested }" />
          </template>
          <span class="app-updater__label">{{ installRequested ? t('appUpdater.installing') : t('appUpdater.install') }}</span>
        </BaseButton>
      </div>
      <progress
        v-if="status?.state === 'downloading'"
        class="app-updater__progress"
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
        v-if="status?.releaseNotes && matchingChannel"
        class="app-updater__notes"
      >
        {{ status.releaseNotes }}
      </p>
      <DiagnosticStatusAction
        v-if="updater.error || status?.error"
        :label="t(unavailable ? 'diagnostics.updaterUnavailable' : 'diagnostics.updateFailed')"
        source="updates"
        :tone="unavailable ? 'info' : 'error'"
      />
      <p v-if="status?.state === 'failed' && status.error?.code === 'download_failed'">
        {{ t('appUpdater.retryHelp') }}
      </p>
      <p
        v-if="!status?.checkSupported || !status?.installSupported || status?.state === 'failed'"
        class="app-updater__muted"
      >
        {{ t(status?.state === 'failed' && status.installSupported ? 'appUpdater.manualRecovery' : 'appUpdater.manual') }} <a
          class="app-updater__url"
          href="https://github.com/nglmercer/TikTools-app/releases"
        >https://github.com/nglmercer/TikTools-app/releases</a>
      </p>
    </div>
  </BaseGroupBox>
</template>

<style scoped>
.app-updater { display: flex; flex-direction: column; gap: var(--space-5); }
.app-updater p { margin: 0; }
.app-updater__facts { display: flex; flex-direction: column; gap: var(--space-2); margin: 0; }
.app-updater__highlight { color: var(--color-success); font-weight: var(--font-weight-semibold); }
.app-updater__muted { color: var(--color-text-subtle); font-size: var(--font-size-sm); }
.app-updater__channel { width: 100%; max-width: 240px; }
.app-updater__actions { display: flex; flex-wrap: wrap; gap: var(--space-3); }
.app-updater__progress { width: 100%; max-width: 360px; accent-color: var(--color-accent); }
.app-updater__notes { white-space: pre-wrap; overflow-wrap: anywhere; }
.app-updater__url { color: var(--color-text-muted); overflow-wrap: anywhere; user-select: text; }
.app-updater__spin { animation: app-updater-spin 1s linear infinite; }
.app-updater__pulse { animation: app-updater-pulse 1.2s ease-in-out infinite; }
@keyframes app-updater-spin { to { transform: rotate(360deg); } }
@keyframes app-updater-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.35; } }
@media (width < 600px) {
  .app-updater__channel { max-width: none; }
  .app-updater__label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); clip-path: inset(50%); white-space: nowrap; }
}
</style>
