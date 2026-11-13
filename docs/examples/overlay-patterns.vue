<script setup lang="ts">
defineOptions({ name: "OverlayPatternsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
const x = ref(20);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BasePanel :title="t.name"><BaseSpinBox v-model="x" :min="0" :max="200" :disabled="disabled" :aria-label="t.count" /><svg viewBox="0 0 320 180" role="img" :aria-label="t.name" style="max-width: 100%; border: 1px solid currentColor"><text :x="x" y="90" fill="currentColor">{{ t.item }}</text></svg></BasePanel></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
