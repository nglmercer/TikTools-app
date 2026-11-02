<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { formatDate } from '@/shared/lib/intl.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';

const props = defineProps<{ registry: string; pluginId: string }>();

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
  <section
    class="remote-details"
    :aria-label="document?.name ?? pluginId"
  >
    <header class="remote-details__header">
      <IconPuzzle
        class="remote-details__icon"
        :size="28"
      />
      <div class="remote-details__heading">
        <h2 class="remote-details__name">
          {{ document?.name ?? pluginId }}
        </h2>
        <span class="remote-details__meta">
          {{ document?.publisher ?? t('packages.review.unknown') }} · {{ registry }}
        </span>
      </div>
      <BaseButton
        variant="primary"
        :disabled="!latest"
        @click="install()"
      >
        {{ t('packages.remote.install') }}
      </BaseButton>
    </header>

    <div class="remote-details__pane">
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
        <dl class="remote-details__facts">
          <dt>{{ t('packages.remote.latest') }}</dt>
          <dd class="remote-details__mono">
            {{ latest ?? t('plugins.details.none') }}
          </dd>
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
        <h3 class="remote-details__heading-sm">
          {{ t('packages.remote.releases') }}
        </h3>
        <ul class="remote-details__releases">
          <li
            v-for="release in releases"
            :key="release.version"
            class="remote-details__release"
          >
            <span class="remote-details__mono">{{ release.version }}</span>
            <span class="remote-details__muted">{{ release.channel }} · {{ formatDate(release.publishedAt * 1000, locale) }}</span>
            <span
              v-if="release.yanked"
              class="remote-details__flag"
            >{{ t('packages.remote.yanked') }}</span>
            <span
              v-if="release.advisory"
              class="remote-details__flag"
            >{{ t(`packages.advisory.${release.advisory.severity}`) }}</span>
            <BaseButton
              size="sm"
              :disabled="release.yanked"
              @click="install(release.version)"
            >
              {{ t('packages.remote.installVersion') }}
            </BaseButton>
          </li>
        </ul>
      </template>
    </div>
  </section>
</template>

<style scoped>
.remote-details {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
  min-height: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.remote-details__header {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
}

.remote-details__icon {
  flex: none;
  color: var(--color-accent);
}

.remote-details__heading {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.remote-details__name {
  margin: 0;
  overflow: hidden;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remote-details__meta,
.remote-details__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.remote-details__pane {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-5);
  min-height: 0;
  padding: var(--space-5);
  overflow: auto;
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

.remote-details__heading-sm {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
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
