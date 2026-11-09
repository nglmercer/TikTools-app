<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAppUpdaterStore } from '../model/updater-store.ts';
import type { UpdateChannel } from '../api/updater-types.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';

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
      <BaseFormGrid>
        <BaseFormRow
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
        </BaseFormRow>
        <BaseFormRow
          for="app-update-automatic"
          :label="t('appUpdater.options')"
        >
          <BaseCheckbox
            id="app-update-automatic"
            :model-value="updater.preferences.automaticChecks"
            :disabled="!updater.preferencesLoaded"
            @update:model-value="updater.setPreference('automaticChecks', $event)"
          >
            {{ t('appUpdater.automaticChecks') }}
          </BaseCheckbox>
          <BaseCheckbox
            :model-value="updater.preferences.notifications"
            :disabled="!updater.preferencesLoaded"
            @update:model-value="updater.setPreference('notifications', $event)"
          >
            {{ t('appUpdater.notifications') }}
          </BaseCheckbox>
        </BaseFormRow>
      </BaseFormGrid>
      <div class="app-updater__actions">
        <BaseButton
          :disabled="busy || !updater.preferencesLoaded || !status?.checkSupported"
          @click="updater.check()"
        >
          {{ t('appUpdater.check') }}
        </BaseButton>
        <BaseButton
          v-if="(status?.state === 'available' || (status?.state === 'failed' && status.error?.code === 'download_failed')) && status.installSupported && matchingChannel"
          :disabled="busy"
          @click="updater.download()"
        >
          {{ t(status?.state === 'failed' ? 'appUpdater.retryDownload' : 'appUpdater.download') }}
        </BaseButton>
        <BaseButton
          v-if="status?.state === 'readyToInstall' && status.installSupported && matchingChannel"
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
@media (width < 600px) { .app-updater__channel { max-width: none; } }
</style>
