<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import { formatBytes } from '@/shared/lib/format-bytes.ts';

/**
 * The signed-in account's plan and what it stores now. Each group shows how many
 * items it holds and how much space they take, against the plan's ceilings.
 */
const { t, locale } = useI18n();
const account = useAccountStore();

const count = computed(() => new Intl.NumberFormat(locale.value));

const bytes = (value: number): string => formatBytes(value, locale.value);

interface Meter {
  key: string;
  label: string;
  used: number;
  max: number;
  format: (value: number) => string;
}

const groups = computed(() => {
  const plan = account.plan;
  if (!plan) return [];
  const { limits, usage } = plan;
  const of = t('account.plan.of');
  const items = (value: number) => count.value.format(value);
  const described = (meters: Meter[]) =>
    meters.map((meter) => ({ ...meter, text: `${meter.format(meter.used)} ${of} ${meter.format(meter.max)}` }));
  return [
    {
      key: 'workspace',
      title: t('account.plan.workspace'),
      meters: described([
        { key: 'records', label: t('account.plan.count'), used: usage.records, max: limits.maxRecords, format: items },
        { key: 'record-bytes', label: t('account.plan.storage'), used: usage.recordBytes, max: limits.maxBytes, format: bytes },
      ]),
    },
    {
      key: 'assets',
      title: t('account.plan.assets'),
      meters: described([
        { key: 'assets', label: t('account.plan.count'), used: usage.assets, max: limits.maxAssets, format: items },
        { key: 'asset-bytes', label: t('account.plan.storage'), used: usage.assetBytes, max: limits.maxAssetsBytes, format: bytes },
      ]),
    },
    {
      key: 'overlays',
      title: t('account.plan.overlays'),
      meters: described([
        { key: 'overlays', label: t('account.plan.count'), used: usage.overlays, max: limits.maxOverlays, format: items },
        { key: 'overlay-bytes', label: t('account.plan.storage'), used: usage.overlayBytes, max: limits.maxOverlayBytes, format: bytes },
      ]),
    },
    {
      key: 'backups',
      title: t('account.plan.backups'),
      meters: described([
        { key: 'backups', label: t('account.plan.count'), used: usage.backups, max: limits.maxBackups, format: items },
        { key: 'backup-bytes', label: t('account.plan.storage'), used: usage.backupBytes, max: limits.maxBackupBytes, format: bytes },
      ]),
    },
  ];
});

/** One entry per area: a count, the space it takes, and one bar for whichever is closer to its ceiling. */
const areas = computed(() => groups.value.map((group) => {
  const [items, storage] = [group.meters[0]!, group.meters[1]!];
  const ratio = Math.max(items.used / Math.max(items.max, 1), storage.used / Math.max(storage.max, 1));
  return { key: group.key, title: group.title, items, storage, ratio: Math.min(1, ratio) };
}));

// On a phone the plan folds to its tightest area; on wider screens it is always open.
const compact = useMediaQuery('(width < 600px)');
const expanded = ref(false);
const open = computed(() => !compact.value || expanded.value);
const tightest = computed(() => areas.value.reduce<(typeof areas.value)[number] | null>(
  (best, area) => (!best || area.ratio > best.ratio ? area : best), null));

onMounted(() => void account.refreshPlan());
watch(
  () => account.status?.state,
  (state) => {
    if (state === 'signed-in') void account.refreshPlan();
  },
);
</script>

<template>
  <BasePanel
    class="cloud-plan"
    data-guide="account-plan"
    :title="t('account.plan.title')"
    header-layout="inline"
    flush
  >
    <template
      v-if="compact && areas.length"
      #actions
    >
      <BaseButton
        size="sm"
        variant="ghost"
        :aria-expanded="open"
        @click="expanded = !expanded"
      >
        {{ open ? t('account.plan.hide') : t('account.plan.show') }}
        <template #icon>
          <IconChevronUp
            v-if="open"
            :size="14"
          />
          <IconChevronDown
            v-else
            :size="14"
          />
        </template>
      </BaseButton>
    </template>
    <template v-if="areas.length">
      <ul
        v-if="open"
        class="cloud-plan__areas"
      >
        <li
          v-for="area in areas"
          :key="area.key"
          class="cloud-plan__area"
        >
          <span class="cloud-plan__label">{{ area.title }}</span>
          <span class="cloud-plan__value">{{ area.items.text }}</span>
          <progress
            class="cloud-plan__bar"
            :value="area.ratio"
            max="1"
            :aria-label="`${area.title} · ${area.items.text} · ${area.storage.text}`"
          />
          <span class="cloud-plan__storage">{{ area.storage.text }}</span>
        </li>
      </ul>
      <div
        v-else-if="tightest"
        class="cloud-plan__area cloud-plan__area--summary"
      >
        <span class="cloud-plan__label">{{ tightest.title }}</span>
        <span class="cloud-plan__value">{{ tightest.items.text }}</span>
        <progress
          class="cloud-plan__bar"
          :value="tightest.ratio"
          max="1"
          :aria-label="`${tightest.title} · ${tightest.items.text}`"
        />
      </div>
    </template>
    <p
      v-else-if="account.planError"
      class="cloud-plan__message"
      role="status"
    >
      {{ t('account.plan.unavailable') }}
    </p>
    <p
      v-else
      class="cloud-plan__message"
    >
      {{ t('account.plan.loading') }}
    </p>
  </BasePanel>
</template>

<style scoped>
/* Width, not viewport: the same markup is a list in the 300px sidebar, 2x2 on a phone and one row on a tablet. */
.cloud-plan { container-type: inline-size; }
.cloud-plan__areas { display: grid; margin: 0; padding: 0; list-style: none; }
.cloud-plan__area {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--space-1) var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}
.cloud-plan__areas > .cloud-plan__area:last-child { border-bottom: 0; }
.cloud-plan__label { overflow: hidden; font-weight: var(--font-weight-semibold); text-overflow: ellipsis; white-space: nowrap; }
.cloud-plan__value { font-variant-numeric: tabular-nums; white-space: nowrap; }
.cloud-plan__bar { grid-column: 1 / -1; width: 100%; height: 4px; border: 0; appearance: none; background: var(--color-bg-control); accent-color: var(--color-accent, #6366f1); }
.cloud-plan__bar::-webkit-progress-bar { background: var(--color-bg-control); }
.cloud-plan__bar::-webkit-progress-value { background: var(--color-accent, #6366f1); }
.cloud-plan__bar::-moz-progress-bar { background: var(--color-accent, #6366f1); }
.cloud-plan__storage { grid-column: 1 / -1; color: var(--color-text-muted); font-size: var(--font-size-xs); font-variant-numeric: tabular-nums; }
.cloud-plan__message { margin: 0; padding: var(--space-4); color: var(--color-text-muted); }
@container (340px <= width) {
  /* Tiles: the label sits above its count so long names are never cut off. */
  .cloud-plan__area { grid-template-columns: minmax(0, 1fr); }
  .cloud-plan__value { order: 2; }
  .cloud-plan__bar { order: 3; }
  .cloud-plan__storage { order: 4; }
  .cloud-plan__areas { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .cloud-plan__area { border-right: var(--border-width) solid var(--color-border-subtle); }
  .cloud-plan__areas > .cloud-plan__area:nth-child(even) { border-right: 0; }
  .cloud-plan__areas > .cloud-plan__area:nth-last-child(-n + 2) { border-bottom: 0; }
}
@container (560px <= width) {
  .cloud-plan__areas { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .cloud-plan__areas > .cloud-plan__area:nth-child(even) { border-right: var(--border-width) solid var(--color-border-subtle); }
  .cloud-plan__areas > .cloud-plan__area:last-child { border-right: 0; }
  .cloud-plan__areas > .cloud-plan__area { border-bottom: 0; }
}
</style>
