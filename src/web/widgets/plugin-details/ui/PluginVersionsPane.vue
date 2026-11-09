<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutoUpdate } from '@/entities/plugin-package/api/package-types.ts';
import { originDetail, originLabelKey } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { formatDate } from '@/shared/lib/intl.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';

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

    <section
      v-if="receipt && fromRegistry"
      class="versions-pane__section"
    >
      <h3 class="versions-pane__heading">
        {{ t('packages.policy.title') }}
      </h3>
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
    </section>

    <section
      v-if="versions?.retained.length"
      class="versions-pane__section"
    >
      <h3 class="versions-pane__heading">
        {{ t('packages.versions.retained') }}
      </h3>
      <ul class="versions-pane__list">
        <li
          v-for="entry in versions.retained"
          :key="entry.version"
        >
          <span class="versions-pane__mono">{{ entry.version }}</span>
          <span class="versions-pane__muted">{{ formatDate(entry.retainedAt * 1000, locale) }}</span>
          <BaseButton
            size="sm"
            :disabled="busy"
            @click="packages.rollback(pluginId, entry.version)"
          >
            {{ t('packages.versions.rollback') }}
          </BaseButton>
        </li>
      </ul>
    </section>

    <section
      v-if="versions?.available.length"
      class="versions-pane__section"
    >
      <h3 class="versions-pane__heading">
        {{ t('packages.versions.available', { registry: versions.registry ?? '' }) }}
      </h3>
      <p class="versions-pane__muted">
        {{ t('packages.liveStatus.hint') }}
      </p>
      <BaseButton
        size="sm"
        @click="load"
      >
        {{ t('packages.liveStatus.refresh') }}
      </BaseButton>
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
          <BaseButton
            size="sm"
            :disabled="busy || !release.compatible || release.liveStatus !== 'approved' || release.yanked || release.version === receipt?.version"
            @click="installVersion(release.version)"
          >
            {{ release.version === receipt?.version ? t('packages.versions.current') : t('packages.remote.installVersion') }}
          </BaseButton>
        </li>
      </ul>
    </section>
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
  gap: var(--space-5);
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

.versions-pane__section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.versions-pane__heading {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
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
  gap: var(--space-4);
}

.versions-pane__list li > .versions-pane__muted {
  flex: 1;
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
