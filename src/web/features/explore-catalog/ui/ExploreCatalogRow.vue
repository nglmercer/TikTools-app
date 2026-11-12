<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { CatalogItemSummary } from '@/entities/catalog/api/catalog-api.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
defineProps<{ item: CatalogItemSummary; installed: boolean; importing: boolean }>();
const emit = defineEmits<{ open: []; install: []; import: [] }>();
const { t } = useI18n();
function kindLabel(item: CatalogItemSummary): string {
  return t(`unifiedCatalog.kinds.${item.kind}`);
}

function sub(item: CatalogItemSummary): string {
  return [item.publisher, item.summary].filter((part): part is string => Boolean(part)).join(' · ') || kindLabel(item);
}

</script>

<template>
  <li
    class="explore-list__row"
    :class="{ 'explore-list__row--off': item.yanked }"
    :data-catalog-key="`${item.kind}:${item.id}`"
  >
    <span
      v-tooltip="kindLabel(item)"
      class="explore-list__kind"
      role="img"
      :aria-label="kindLabel(item)"
    >
      <IconPuzzle v-if="item.kind === 'plugin'" />
      <IconFlow v-else-if="item.kind === 'template'" />
      <IconLayers v-else />
    </span>
    <button
      v-if="item.kind === 'plugin'"
      class="explore-list__open"
      type="button"
      @click="emit('open')"
    >
      <span class="explore-list__name">{{ item.title }}</span>
      <span class="explore-list__sub">{{ sub(item) }}</span>
    </button>
    <div
      v-else
      class="explore-list__open explore-list__open--static"
    >
      <span class="explore-list__name">{{ item.title }}</span>
      <span class="explore-list__sub">{{ sub(item) }}</span>
    </div>

    <span
      v-if="item.yanked"
      class="explore-list__chip explore-list__control"
    >
      <IconWarning />
      {{ t('unifiedCatalog.yanked') }}
    </span>
    <span
      v-if="installed"
      v-tooltip="t('exploreCatalog.installedBadge')"
      class="explore-list__meta explore-list__control"
      role="img"
      :aria-label="t('exploreCatalog.installedBadge')"
    >
      <IconCheck :size="14" />
    </span>
    <span
      v-if="item.version"
      class="explore-list__meta explore-list__mono"
    >{{ item.version }}</span>

    <BaseIconButton
      v-if="item.kind === 'plugin' && !installed && !item.yanked"
      class="explore-list__control"
      :label="t('exploreCatalog.install', { name: item.title })"
      :hint="t('packages.remote.install')"
      variant="default"
      size="sm"
      @click="emit('install')"
    >
      <IconDownload />
    </BaseIconButton>
    <BaseIconButton
      v-else-if="item.kind !== 'plugin'"
      class="explore-list__control"
      :label="t(item.kind === 'template' ? 'exploreCatalog.importTemplate' : 'exploreCatalog.importProfile', { name: item.title })"
      variant="default"
      size="sm"
      :disabled="importing"
      @click="emit('import')"
    >
      <IconDownload />
    </BaseIconButton>
    <IconChevronRight
      v-if="item.kind === 'plugin'"
      class="explore-list__chevron"
    />
  </li>
</template>
