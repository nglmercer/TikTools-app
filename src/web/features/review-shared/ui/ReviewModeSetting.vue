<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import { resolutionMode, setResolutionMode, type ResolutionMode } from '../model/resolution-preference.ts';

const { t } = useI18n();
const client = useControlClient();
const options = computed(() => [
  { value: 'automatic', label: t('settingsLayout.resolution.automatic') },
  { value: 'manual', label: t('settingsLayout.resolution.manual') },
]);
const selected = computed<ResolutionMode>({
  get: () => resolutionMode.value,
  set: (value) => { void setResolutionMode(client, value); },
});
</script>

<template>
  <BaseComboBox
    v-model="selected"
    :options="options"
    :aria-label="t('settingsLayout.resolution.label')"
  />
</template>
