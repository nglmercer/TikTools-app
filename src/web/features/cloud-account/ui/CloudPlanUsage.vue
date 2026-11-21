<script setup lang="ts">
import { computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';

/**
 * The signed-in account's plan and what it stores now. Each group shows how many
 * items it holds and how much space they take, against the plan's ceilings.
 */
const { t, locale } = useI18n();
const account = useAccountStore();

const count = computed(() => new Intl.NumberFormat(locale.value));

function bytes(value: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : 1;
  return `${new Intl.NumberFormat(locale.value, { maximumFractionDigits: digits, minimumFractionDigits: digits }).format(amount)} ${units[unit]}`;
}

const planName = computed(() => {
  const name = account.plan?.plan ?? '';
  return name ? name.charAt(0).toUpperCase() + name.slice(1) : '';
});

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
  ];
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
  <section
    class="cloud-plan"
    :aria-label="t('account.plan.title')"
  >
    <header class="cloud-plan__header">
      <h3 class="cloud-plan__title">
        {{ t('account.plan.title') }}
      </h3>
      <span
        v-if="account.plan"
        class="cloud-plan__badge"
      >{{ planName }}</span>
    </header>
    <div
      v-if="groups.length"
      class="cloud-plan__groups"
    >
      <article
        v-for="group in groups"
        :key="group.key"
        class="cloud-plan__group"
      >
        <h4 class="cloud-plan__group-title">
          {{ group.title }}
        </h4>
        <div
          v-for="meter in group.meters"
          :key="meter.key"
          class="cloud-plan__meter"
        >
          <div class="cloud-plan__meter-head">
            <span>{{ meter.label }}</span>
            <span class="cloud-plan__meter-value">{{ meter.text }}</span>
          </div>
          <progress
            class="cloud-plan__bar"
            :value="meter.used"
            :max="Math.max(meter.max, 1)"
            :aria-label="`${group.title} · ${meter.label}`"
          />
        </div>
      </article>
    </div>
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
  </section>
</template>

<style scoped>
.cloud-plan { display: grid; gap: var(--space-4); }
.cloud-plan__header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); }
.cloud-plan__title { margin: 0; font-size: var(--font-size-md); }
.cloud-plan__badge {
  padding: 2px var(--space-3);
  border-radius: 999px;
  background: var(--color-accent-soft, rgba(99, 102, 241, 0.18));
  color: var(--color-accent, inherit);
  font-size: var(--font-size-sm);
  font-weight: 600;
}
.cloud-plan__groups { display: grid; grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); gap: var(--space-4); }
.cloud-plan__group { display: grid; gap: var(--space-3); padding: var(--space-3); border-radius: var(--radius-md, 8px); background: var(--color-bg-raised); }
.cloud-plan__group-title { margin: 0; font-size: var(--font-size-sm); color: var(--color-text-muted); }
.cloud-plan__meter { display: grid; gap: var(--space-1); }
.cloud-plan__meter-head { display: flex; justify-content: space-between; gap: var(--space-2); font-size: var(--font-size-sm); }
.cloud-plan__meter-value { font-variant-numeric: tabular-nums; white-space: nowrap; }
.cloud-plan__bar { width: 100%; height: 6px; accent-color: var(--color-accent, #6366f1); }
.cloud-plan__message { margin: 0; color: var(--color-text-muted); }
</style>
