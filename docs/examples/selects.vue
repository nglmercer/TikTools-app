<script setup lang="ts">
defineOptions({ name: "SelectsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';
const value = ref('one');
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseComboBox v-model="value" :aria-label="t.name" :disabled="disabled" :options="props.state === 'empty' ? [] : [{ value: 'one', label: t.item }, { value: 'two', label: t.long }]" /><BaseSearchSelect v-model="value" :aria-label="t.search" :disabled="disabled" :options="[{ value: 'one', label: t.item }, { value: 'two', label: t.long }]" /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
