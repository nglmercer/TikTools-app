<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutoUpdate } from '@/entities/plugin-package/api/package-types.ts';
import { originDetail, originLabelKey } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { formatDate } from '@/shared/lib/intl.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';

const props = defineProps<{ pluginId: string }>();

const { t, locale } = useI18n();
const packages = usePackagesStore();

const versions = computed(() => packages.versions[props.pluginId]);
const receipt = computed(() => packages.receipts[props.pluginId] ?? versions.value?.receipt);
const error = computed(() => packages.versionsError[props.pluginId]);
const fromRegistry = computed(() => receipt.value?.origin.kind === 'registry');
const busy = computed(() => packages.activeJobs.some((job) => job.pluginId === props.pluginId));

const channelOptions = computed(() => [
  { value: 'stable', label: t('packages.policy.stable') },
  { value: 'beta', label: t('packages.policy.beta') },
]);
const autoOptions = computed(() =>
  (['off', 'notify', 'patch', 'minor'] as const).map((value) => ({ value, label: t(`packages.policy.auto.${value}`) })),
);

const channel = computed({
  get: () => receipt.value?.policy.channel ?? 'stable',
  set: (value: string) => void packages.setPolicy(props.pluginId, { channel: value }),
});
const autoUpdate = computed({
  get: () => receipt.value?.policy.autoUpdate ?? 'notify',
  set: (value: AutoUpdate) => void packages.setPolicy(props.pluginId, { autoUpdate: value }),
});
const pinned = computed({
  get: () => receipt.value?.policy.pinned ?? false,
  set: (value: boolean) => void packages.setPolicy(props.pluginId, { pinned: value }),
});

function load(): void {
  void packages.loadVersions(props.pluginId);
}

function installVersion(version: string): void {
  void packages.reviewSpec({
    kind: 'registry',
    id: props.pluginId,
    version,
    ...(versions.value?.registry ? { registry: versions.value.registry } : {}),
  });
}

function reviewCloudVersion(version: string): void {
  // Re-resolve the configured Cloud source and require a fresh package
  // review. The retained directory cannot prove a Cloud release is still
  // approved, and a legacy receipt must not select another registry.
  void packages.reviewSpec({ kind: 'registry', id: props.pluginId, version });
}

onMounted(load);
watch(() => props.pluginId, load);
</script>

<template>
  <div class="versions-pane">
    <BaseAlert
      v-if="error"
      :title="error"
      compact
    />
    <dl
      v-if="receipt"
      class="versions-pane__facts"
    >
      <dt>{{ t('packages.versions.installed') }}</dt>
      <dd class="versions-pane__mono">
        {{ receipt.version }}
      </dd>
      <dt>{{ t('packages.review.source') }}</dt>
      <dd>
        {{ t(originLabelKey(receipt.origin)) }}
        <span
          v-if="originDetail(receipt.origin)"
          class="versions-pane__mono"
        >{{ originDetail(receipt.origin) }}</span>
      </dd>
      <dt>{{ t('packages.review.signatureLabel') }}</dt>
      <dd>{{ t(`packages.signature.${receipt.signature.status}`, { publisher: receipt.signature.publisher ?? '' }) }}</dd>
      <dt>{{ t('packages.versions.updated') }}</dt>
      <dd>{{ formatDate(receipt.updatedAt * 1000, locale) }}</dd>
    </dl>
    <p
      v-else
      class="versions-pane__muted"
    >
      {{ t('packages.versions.unmanaged') }}
    </p>

    <BaseSection
      v-if="receipt && fromRegistry"
      :title="t('packages.policy.title')"
    >
      <BaseFormGrid>
        <BaseFormRow
          for="plugin-policy-channel"
          :label="t('packages.policy.channel')"
        >
          <BaseComboBox
            id="plugin-policy-channel"
            v-model="channel"
            :options="channelOptions"
          />
        </BaseFormRow>
        <BaseFormRow
          for="plugin-policy-auto"
          :label="t('packages.policy.autoUpdate')"
          :hint="t('packages.policy.autoHint')"
        >
          <BaseComboBox
            id="plugin-policy-auto"
            v-model="autoUpdate"
            :options="autoOptions"
          />
        </BaseFormRow>
      </BaseFormGrid>
      <BaseCheckbox v-model="pinned">
        {{ t('packages.policy.pinned') }}
      </BaseCheckbox>
    </BaseSection>

    <BaseSection
      v-if="versions?.retained.length"
      :title="t('packages.versions.retained')"
    >
      <ul class="versions-pane__list">
        <li
          v-for="entry in versions.retained"
          :key="entry.version"
        >
          <span class="versions-pane__mono">{{ entry.version }}</span>
          <span class="versions-pane__muted">{{ formatDate(entry.retainedAt * 1000, locale) }}</span>
          <BaseIconButton
            :label="t(fromRegistry ? 'packages.versions.restoreLocalVersion' : 'packages.versions.restoreVersion', { version: entry.version })"
            variant="default"
            size="sm"
            :disabled="busy"
            @click="packages.rollback(pluginId, entry.version)"
          >
            <IconUndo />
          </BaseIconButton>
          <BaseIconButton
            v-if="fromRegistry"
            :label="t('packages.versions.reviewCloudVersion', { version: entry.version })"
            :hint="t('packages.versions.cloudRestoreHint')"
            variant="default"
            size="sm"
            :disabled="busy"
            @click="reviewCloudVersion(entry.version)"
          >
            <IconGlobe />
          </BaseIconButton>
        </li>
      </ul>
    </BaseSection>

    <BaseSection
      v-if="versions?.available.length"
      :title="t('packages.versions.available', { registry: versions.registry ?? '' })"
    >
      <template #actions>
        <BaseIconButton
          :label="t('packages.liveStatus.refresh')"
          :hint="t('packages.liveStatus.hint')"
          size="sm"
          @click="load"
        >
          <IconRefresh />
        </BaseIconButton>
      </template>
      <ul class="versions-pane__list">
        <li
          v-for="release in versions.available"
          :key="release.version"
        >
          <span class="versions-pane__mono">{{ release.version }}</span>
          <span class="versions-pane__muted">
            {{ release.channel }} · {{ formatDate(release.publishedAt * 1000, locale) }}
            <template v-if="!release.compatible"> · {{ t('packages.versions.incompatible') }}</template>
            <template v-if="release.yanked"> · {{ t('packages.remote.yanked') }}</template>
            · {{ t(`packages.liveStatus.${release.liveStatus ?? 'unknown'}`) }}
          </span>
          <IconCheck
            v-if="release.version === receipt?.version"
            class="versions-pane__current"
            :label="t('packages.versions.current')"
          />
          <BaseIconButton
            v-else
            :label="t('plugins.catalog.installPlugin', { name: release.version })"
            variant="default"
            size="sm"
            :disabled="busy || !release.compatible || release.liveStatus !== 'approved' || release.yanked"
            @click="installVersion(release.version)"
          >
            <IconDownload />
          </BaseIconButton>
        </li>
      </ul>
    </BaseSection>
    <p
      v-else-if="versions?.registryError && fromRegistry"
      class="versions-pane__muted"
    >
      {{ versions.registryError }}
    </p>
  </div>
</template>

<style scoped>
.versions-pane {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.versions-pane__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-3) var(--space-6);
  margin: 0;
}

.versions-pane__facts dt {
  color: var(--color-text-muted);
}

.versions-pane__facts dd {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
  overflow-wrap: anywhere;
}

.versions-pane__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.versions-pane__list li {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.versions-pane__list li > .versions-pane__muted {
  flex: 1;
}

.versions-pane__current {
  color: var(--color-success);
}

.versions-pane__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.versions-pane__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}
</style>
