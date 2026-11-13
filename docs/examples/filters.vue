<script setup lang="ts">
defineOptions({ name: "FiltersExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import CollectionToolbar from '@/shared/ui/collection-toolbar/CollectionToolbar.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
const query = ref(''); const enabled = ref(false);
</script>
<template>
  <div class="component-example" :data-state="props.state"><CollectionToolbar :label="t.search" :count="'3'"><template #search><BaseTextField v-model="query" type="search" :aria-label="t.search" :disabled="disabled" /></template><BaseCheckbox v-model="enabled" :aria-label="t.enabled" :disabled="disabled" /></CollectionToolbar><output>{{ query }}</output></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
