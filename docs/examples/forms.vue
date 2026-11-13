<script setup lang="ts">
defineOptions({ name: "FormsExample" });
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { exampleText } from './text.ts';
const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
const value = ref('');
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseFormRow v-slot="{ controlAttrs }" :label="t.name" :hint="t.hint" :error="props.state === 'error' ? t.error : undefined" required stacked><BaseTextField v-bind="controlAttrs" v-model="value" :disabled="disabled" /></BaseFormRow></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
