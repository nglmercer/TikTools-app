<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';

/** The signed-in account's plan and what it stores now, against the plan's ceilings. */
const { t } = useI18n();
const account = useAccountStore();

/** Human byte count with one decimal above one kilobyte. */
function formatBytes(value: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  return `${amount.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

const count = (value: number) => String(value);

const meters = computed(() => {
  const plan = account.plan;
  if (!plan) return [];
  const { limits, usage } = plan;
  const of = t('account.plan.of');
  const rows = [
    { key: 'records', label: t('account.plan.records'), used: usage.records, max: limits.maxRecords, format: count },
    { key: 'record-bytes', label: t('account.plan.storage'), used: usage.recordBytes, max: limits.maxBytes, format: formatBytes },
    { key: 'assets', label: t('account.plan.assets'), used: usage.assets, max: limits.maxAssets, format: count },
    { key: 'asset-bytes', label: `${t('account.plan.assets')} · ${t('account.plan.storage')}`, used: usage.assetBytes, max: limits.maxAssetsBytes, format: formatBytes },
    { key: 'overlays', label: t('account.plan.overlays'), used: usage.overlays, max: limits.maxOverlays, format: count },
    { key: 'overlay-bytes', label: `${t('account.plan.overlays')} · ${t('account.plan.storage')}`, used: usage.overlayBytes, max: limits.maxOverlayBytes, format: formatBytes },
  ];
  return rows.map((row) => ({
    ...row,
    text: `${row.format(row.used)} ${of} ${row.format(row.max)}`,
  }));
});

onMounted(() => void account.refreshPlan());
watch(
  () => account.status?.state,
  (state) => {
    if (state === 'signed-in') void account.refreshPlan();
  },
);
</script>

<template>
  <div class="cloud-plan">
    <p
      v-if="account.plan"
      class="cloud-plan__name"
    >
      {{ t('account.plan.plan') }}: <strong>{{ account.plan.plan }}</strong>
    </p>
    <dl
      v-if="meters.length"
      class="cloud-plan__meters"
    >
      <template
        v-for="meter in meters"
        :key="meter.key"
      >
        <dt>{{ meter.label }}</dt>
        <dd>
          <progress
            :value="meter.used"
            :max="Math.max(meter.max, 1)"
            :aria-label="meter.label"
          >
            {{ meter.text }}
          </progress>
          <span>{{ meter.text }}</span>
        </dd>
      </template>
    </dl>
    <p
      v-else-if="account.planError"
      class="cloud-plan__error"
    >
      {{ t('account.plan.unavailable') }}
    </p>
  </div>
</template>

<style scoped>
.cloud-plan__meters { display: grid; grid-template-columns: max-content 1fr; gap: var(--space-2) var(--space-4); align-items: center; margin: 0; }
.cloud-plan__meters dt { color: var(--color-text-muted, inherit); }
.cloud-plan__meters dd { display: flex; align-items: center; gap: var(--space-3); margin: 0; min-width: 0; }
.cloud-plan__meters progress { flex: 1 1 auto; min-width: 6rem; height: 0.5rem; }
.cloud-plan__meters span { font-family: var(--font-mono, monospace); white-space: nowrap; }
.cloud-plan__name { margin: 0 0 var(--space-3); }
</style>
