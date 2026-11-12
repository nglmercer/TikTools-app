<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutoUpdate, RegistryDefaults } from '@/entities/plugin-package/api/package-types.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';

const { t } = useI18n();
const packages = usePackagesStore();
const channelOptions = computed(() => [
  { value: 'stable', label: t('packages.policy.stable') },
  { value: 'beta', label: t('packages.policy.beta') },
]);
const autoOptions = computed(() =>
  (['off', 'notify', 'patch', 'minor'] as const).map((value) => ({ value, label: t(`packages.policy.auto.${value}`) })),
);
function update<K extends keyof RegistryDefaults>(key: K, value: RegistryDefaults[K]): void {
  if (!packages.defaults) return;
  void packages.saveDefaults({ ...packages.defaults, [key]: value });
}
const channel = computed({ get: () => packages.defaults?.channel ?? 'stable', set: (value: string) => update('channel', value) });
const autoUpdate = computed({ get: () => packages.defaults?.autoUpdate ?? 'notify', set: (value: AutoUpdate) => update('autoUpdate', value) });
const interval = computed({ get: () => packages.defaults?.checkIntervalHours ?? 12, set: (value: number) => update('checkIntervalHours', Math.round(value)) });
const allowSideload = computed({ get: () => packages.defaults?.allowSideload ?? true, set: (value: boolean) => update('allowSideload', value) });
onMounted(() => void packages.loadRegistries());
</script>

<template>
  <div class="plugin-update-policy">
    <BaseAlert
      v-if="packages.registryError"
      :title="packages.registryError"
      compact
    />
    <BaseFormGrid>
      <BaseFormRow
        for="plugin-update-policy-channel"
        :label="t('packages.policy.channel')"
      >
        <BaseComboBox
          id="plugin-update-policy-channel"
          v-model="channel"
          :options="channelOptions"
          :disabled="!packages.defaults"
        />
      </BaseFormRow>
      <BaseFormRow
        for="plugin-update-policy-auto"
        :label="t('packages.policy.autoUpdate')"
        :hint="t('packages.policy.autoHint')"
      >
        <BaseComboBox
          id="plugin-update-policy-auto"
          v-model="autoUpdate"
          :options="autoOptions"
          :disabled="!packages.defaults"
        />
      </BaseFormRow>
      <BaseFormRow
        for="plugin-update-policy-interval"
        :label="t('packages.sources.interval')"
      >
        <BaseSpinBox
          id="plugin-update-policy-interval"
          v-model="interval"
          :min="1"
          :max="168"
          :suffix="t('packages.sources.hours')"
          :disabled="!packages.defaults"
        />
      </BaseFormRow>
    </BaseFormGrid>
    <BaseSwitch
      v-model="allowSideload"
      :disabled="!packages.defaults"
    >
      {{ t('packages.sources.allowSideload') }}
    </BaseSwitch>
  </div>
</template>

<style scoped>
.plugin-update-policy {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
</style>
