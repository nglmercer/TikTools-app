<script setup lang="ts">
defineOptions({ name: "SwitchesExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
const value = ref(false);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseSwitch v-model="value" :disabled="disabled" :aria-label="t.enabled" /><output>{{ value ? t.enabled : t.disabled }}</output></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
