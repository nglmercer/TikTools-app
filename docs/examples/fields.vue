<script setup lang="ts">
defineOptions({ name: "FieldsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
const value = ref(''); const number = ref(5);
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseTextField v-model="value" :aria-label="t.name" :placeholder="t.name" :disabled="disabled" :invalid="props.state === 'error'" /><BaseSpinBox v-model="number" :min="0" :max="100" :disabled="disabled" :aria-label="t.count" /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
