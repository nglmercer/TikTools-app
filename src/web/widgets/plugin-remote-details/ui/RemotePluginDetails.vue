<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { formatDate } from '@/shared/lib/intl.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import DialogRefreshBar from '@/shared/ui/dialog/DialogRefreshBar.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

const props = defineProps<{ registry: string; pluginId: string }>();
const emit = defineEmits<{ back: [] }>();

const { t, locale } = useI18n();
const packages = usePackagesStore();

const key = computed(() => `${props.registry}/${props.pluginId}`);
const view = computed(() => packages.remotePackages[key.value]);
const document = computed(() => view.value?.document);
const error = computed(() => packages.remoteErrors[key.value]);
const releases = computed(() => [...(document.value?.releases ?? [])].reverse());
const latest = computed(() => document.value?.['dist-tags']?.stable ?? releases.value.find((release) => !release.yanked)?.version);

function load(): void {
  void packages.loadRemotePackage(props.registry, props.pluginId);
}

function install(version?: string): void {
  void packages.reviewSpec({
    kind: 'registry',
    id: props.pluginId,
    registry: props.registry,
    ...(version ? { version } : {}),
  });
}

onMounted(load);
watch(key, load);
</script>

<template>
  <BasePanel
    class="remote-details"
    :title="document?.name ?? pluginId"
    flush
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        size="sm"
        @click="emit('back')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
    <template
      v-if="latest"
      #status
    >
      <span
        v-tooltip="t('packages.remote.latest')"
        class="remote-details__chip remote-details__mono"
      >{{ latest }}</span>
    </template>
    <template #actions>
      <BaseButton
        variant="primary"
        :disabled="!latest || view?.liveStatuses?.[latest] !== 'approved'"
        @click="install(latest)"
      >
        <template #icon>
          <IconDownload />
        </template>
        {{ t('packages.remote.install') }}
      </BaseButton>
    </template>

    <div class="remote-details__pane">
      <DialogRefreshBar
        :label="t('packages.liveStatus.refresh')"
        @refresh="load"
      >
        {{ document?.publisher ?? t('packages.review.unknown') }} · {{ registry }}
      </DialogRefreshBar>
      <BaseAlert
        v-if="error"
        :title="t('packages.remote.loadFailed')"
      >
        <p class="remote-details__muted">
          {{ error }}
        </p>
      </BaseAlert>
      <p
        v-else-if="!document"
        class="remote-details__muted"
        role="status"
      >
        {{ t('packages.review.loading') }}
      </p>
      <template v-else>
        <BaseMarkdown
          v-if="document.description"
          :source="document.description"
        />
        <dl
          v-if="document.license || document.categories?.length || document.homepage || document.repository"
          class="remote-details__facts"
        >
          <dt v-if="document.license">
            {{ t('packages.review.license') }}
          </dt>
          <dd v-if="document.license">
            {{ document.license }}
          </dd>
          <dt v-if="document.categories?.length">
            {{ t('packages.remote.categories') }}
          </dt>
          <dd v-if="document.categories?.length">
            <span
              v-for="category in document.categories"
              :key="category"
              class="remote-details__chip"
            >{{ t(`packages.categories.${category}`) }}</span>
          </dd>
          <dt v-if="document.homepage">
            {{ t('packages.remote.homepage') }}
          </dt>
          <dd v-if="document.homepage">
            <a
              :href="document.homepage"
              target="_blank"
              rel="noopener noreferrer"
            >{{ document.homepage }}</a>
          </dd>
          <dt v-if="document.repository">
            {{ t('packages.remote.repository') }}
          </dt>
          <dd v-if="document.repository">
            <a
              :href="document.repository"
              target="_blank"
              rel="noopener noreferrer"
            >{{ document.repository }}</a>
          </dd>
        </dl>
        <BaseSection :title="t('packages.remote.releases')">
          <ul class="remote-details__releases">
            <li
              v-for="release in releases"
              :key="release.version"
              class="remote-details__release"
            >
              <span class="remote-details__mono">{{ release.version }}</span>
              <span class="remote-details__muted">{{ release.channel }} · {{ formatDate(release.publishedAt * 1000, locale) }}</span>
              <span class="remote-details__flag">{{ t(`packages.liveStatus.${view?.liveStatuses?.[release.version] ?? 'unknown'}`) }}</span>
              <span
                v-if="release.yanked"
                class="remote-details__flag"
              >{{ t('packages.remote.yanked') }}</span>
              <span
                v-if="release.advisory"
                class="remote-details__flag"
              >{{ t(`packages.advisory.${release.advisory.severity}`) }}</span>
              <BaseIconButton
                :label="t('plugins.catalog.installPlugin', { name: release.version })"
                variant="default"
                size="sm"
                :disabled="release.yanked || view?.liveStatuses?.[release.version] !== 'approved'"
                @click="install(release.version)"
              >
                <IconDownload />
              </BaseIconButton>
            </li>
          </ul>
        </BaseSection>
      </template>
    </div>
  </BasePanel>
</template>

<style scoped>
.remote-details {
  flex: 1;
}

.remote-details__pane {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
}

.remote-details__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.remote-details__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-3) var(--space-6);
  margin: 0;
}

.remote-details__facts dt {
  color: var(--color-text-muted);
}

.remote-details__facts dd {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
  overflow-wrap: anywhere;
}

.remote-details__chip {
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  line-height: 20px;
}

.remote-details__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.remote-details__releases {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.remote-details__release {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.remote-details__release > .remote-details__muted {
  flex: 1;
}

.remote-details__flag {
  color: var(--color-warning);
  font-size: var(--font-size-sm);
}
</style>
