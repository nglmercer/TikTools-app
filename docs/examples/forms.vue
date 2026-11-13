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
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import type { JsonObject } from '../../src/automation/types.ts';
const value = ref('');
const start = ref('');
const end = ref('');
const data = ref<JsonObject>({ mode: 'text', message: '' });
const schema = computed<JsonObject>(() => ({ type: 'object', properties: { mode: { type: 'string', title: t.value.name, enum: ['text', 'media'] }, message: { type: 'string', title: t.value.item }, path: { type: 'string', title: t.value.name } } }));
const uiHints: JsonObject = { fields: { message: { kind: 'textarea', showIf: { key: 'mode', equals: ['text'] } }, path: { kind: 'media', showIf: { key: 'mode', equals: ['media'] } } } };
</script>
<template>
  <div class="component-example" :data-state="props.state"><BaseFormRow v-slot="{ controlAttrs }" :label="t.name" :hint="t.hint" :error="props.state === 'error' ? t.error : undefined" required stacked><BaseTextField v-bind="controlAttrs" v-model="value" :disabled="disabled" /></BaseFormRow><BaseFormRow :label="t.name" :hint="t.hint" group stacked><BaseTextField v-model="start" :aria-label="`${t.name} 1`" :disabled="disabled" /><BaseTextField v-model="end" :aria-label="`${t.name} 2`" :disabled="disabled" /></BaseFormRow><BaseSchemaForm v-model="data" :schema="schema" :ui-hints="uiHints" :field-errors="props.state === 'error' ? { message: t.error } : {}" :disabled="disabled" /></div>
</template>
<style scoped>
.component-example { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; min-width: 0; padding: 12px; }
.component-example > :deep(*) { max-width: 100%; }
</style>
