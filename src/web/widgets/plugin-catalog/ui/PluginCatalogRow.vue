<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import type { PluginsPanelTab } from '@/entities/plugin/lib/plugin-catalog.ts';
import type { PluginCommand } from '@/entities/plugin/lib/plugin-menu.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import type { CatalogRow } from '../model/catalog-types.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';

const props = defineProps<{ row: CatalogRow; tab: PluginsPanelTab; checkingUpdates?: boolean }>();
const emit = defineEmits<{ open: [key: string]; command: [id: string, command: PluginCommand]; update: [id: string, version?: string] }>();
const { t } = useI18n();
function rowName(row: CatalogRow): string {
  if (row.kind === 'local') return localizedText(row.plugin.descriptor.name);
  if (row.kind === 'remote') return row.entry.name;
  return row.plugin ? localizedText(row.plugin.descriptor.name) : row.update.id;
}

function sourceLabel(plugin: PluginStatus): string {
  return t(`plugins.details.sources.${plugin.descriptor.source ?? 'builtin'}`);
}

function rowSub(row: CatalogRow): string {
  if (row.kind === 'remote') return row.entry.publisher ?? row.entry.registry;
  if (row.kind === 'update') return row.update.registry;
  return props.tab === 'installed' ? `${sourceLabel(row.plugin)} · ${row.plugin.descriptor.version}` : sourceLabel(row.plugin);
}


</script>

<template>
  <li
    class="plugin-catalog__row"
    :class="{ 'plugin-catalog__row--off': row.kind === 'local' && row.plugin.installed && !row.plugin.enabled }"
    :data-plugin-key="row.key"
  >
    <BaseSwitch
      v-if="row.kind === 'local' && row.plugin.installed"
      class="plugin-catalog__control"
      :model-value="row.plugin.enabled"
      :aria-label="t('plugins.catalog.enable', { name: rowName(row) })"
      @update:model-value="emit('command', row.plugin.descriptor.id, 'toggle-enabled')"
    />
    <button
      class="plugin-catalog__open"
      type="button"
      @click="emit('open', row.key)"
    >
      <span class="plugin-catalog__name">{{ rowName(row) }}</span>
      <span class="plugin-catalog__sub">{{ rowSub(row) }}</span>
    </button>

    <template v-if="row.kind === 'local'">
      <span
        v-if="tab === 'installed' && !row.plugin.available"
        v-tooltip="row.plugin.unavailableReason ?? t('plugins.state.unavailable')"
        class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
      >
        <IconWarning />
        {{ t('plugins.catalog.unavailable') }}
      </span>
      <template v-if="tab === 'installed'">
        <span
          v-tooltip="t('plugins.details.actions')"
          class="plugin-catalog__meta plugin-catalog__control"
          role="img"
          :aria-label="`${t('plugins.details.actions')}: ${row.plugin.descriptor.actionTypeIds.length}`"
        >
          <IconPlay :size="14" />{{ row.plugin.descriptor.actionTypeIds.length }}
        </span>
        <span
          v-tooltip="t('plugins.details.events')"
          class="plugin-catalog__meta plugin-catalog__control"
          role="img"
          :aria-label="`${t('plugins.details.events')}: ${row.plugin.descriptor.eventTypeIds.length}`"
        >
          <IconZap :size="14" />{{ row.plugin.descriptor.eventTypeIds.length }}
        </span>
      </template>
      <template v-else>
        <span class="plugin-catalog__meta plugin-catalog__mono">{{ row.plugin.descriptor.version }}</span>
        <BaseIconButton
          class="plugin-catalog__control"
          :label="t('plugins.catalog.installPlugin', { name: rowName(row) })"
          variant="default"
          size="sm"
          :disabled="!row.plugin.available"
          @click="emit('command', row.plugin.descriptor.id, 'install')"
        >
          <IconDownload />
        </BaseIconButton>
      </template>
    </template>

    <template v-else-if="row.kind === 'remote'">
      <span class="plugin-catalog__meta plugin-catalog__mono">{{ row.entry.latest }}</span>
      <BaseIconButton
        class="plugin-catalog__control"
        :label="t('plugins.catalog.installPlugin', { name: rowName(row) })"
        :hint="t('packages.remote.install')"
        variant="default"
        size="sm"
        @click="emit('open', row.key)"
      >
        <IconDownload />
      </BaseIconButton>
    </template>

    <template v-else>
      <span
        v-if="row.update.currentStatus !== 'approved'"
        class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
      >
        {{ t(`packages.liveStatus.${row.update.currentStatus ?? 'unknown'}`) }}
      </span>
      <span
        v-if="row.update.advisory"
        class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
      >{{ t(`packages.advisory.${row.update.advisory.severity}`) }}</span>
      <span
        v-if="row.update.latest && row.update.latestStatus !== 'approved'"
        class="plugin-catalog__chip plugin-catalog__chip--warning plugin-catalog__control"
      >
        {{ t(`packages.liveStatus.${row.update.latestStatus ?? 'unknown'}`) }}
      </span>
      <span class="plugin-catalog__meta plugin-catalog__mono">
        {{ row.update.latest ? `${row.update.current} → ${row.update.latest}` : row.update.current }}
      </span>
      <BaseIconButton
        v-if="row.update.latest"
        class="plugin-catalog__control"
        :label="t('plugins.catalog.update', { name: rowName(row) })"
        variant="default"
        size="sm"
        :disabled="checkingUpdates || row.update.latestStatus !== 'approved'"
        @click="emit('update', row.update.id, row.update.latest)"
      >
        <IconUpload />
      </BaseIconButton>
    </template>

    <IconChevronRight class="plugin-catalog__chevron" />
  </li>
</template>
