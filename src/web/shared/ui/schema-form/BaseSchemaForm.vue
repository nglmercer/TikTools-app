<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject, JsonValue } from '../../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../../shared/messages.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseDisclosure from '@/shared/ui/disclosure/BaseDisclosure.vue';
import { acceptsValue, describeForm, type SchemaFieldDescriptor, type SchemaOption } from './lib/schema-fields.ts';
import type { SchemaSuggestion } from './lib/schema-suggestion.ts';
import SchemaFieldRows from './SchemaFieldRows.vue';

const props = withDefaults(
  defineProps<{
    schema: JsonObject;
    uiHints?: JsonObject;
    fieldOptions?: Readonly<Record<string, readonly SchemaOption[]>>;
    fieldErrors?: Readonly<Record<string, string>>;
    suggestionsFor?: (key: string) => readonly SchemaSuggestion[];
    openMediaPicker?: OpenMediaPicker;
    disabled?: boolean;
    switches?: boolean;
  }>(),
  { uiHints: undefined, fieldOptions: () => ({}), fieldErrors: () => ({}), suggestionsFor: undefined, openMediaPicker: undefined },
);

const model = defineModel<JsonObject>({ required: true });
const { t, locale } = useI18n();
const formId = useId();
const advancedOpen = ref(false);

const groups = computed(() => {
  void locale.value;
  return describeForm(props.schema, props.uiHints, model.value, props.fieldOptions, localizedText);
});

function update(field: SchemaFieldDescriptor, value: JsonValue): void {
  if (acceptsValue(field, value)) model.value = { ...model.value, [field.key]: value };
}
</script>

<template>
  <div
    class="base-schema-form"
    data-schema-form
  >
    <SchemaFieldRows
      v-if="groups.basic.length"
      :fields="groups.basic"
      :values="model"
      :form-id="formId"
      :field-errors="fieldErrors"
      :suggestions-for="suggestionsFor"
      :open-media-picker="openMediaPicker"
      :disabled="disabled"
      :switches="switches"
      @update="update"
    />
    <BaseDisclosure
      v-if="groups.advanced.length"
      v-model:open="advancedOpen"
      :title="t('schemaForm.advanced')"
      :count="groups.advanced.length"
    >
      <SchemaFieldRows
        :fields="groups.advanced"
        :values="model"
        :form-id="formId"
        :field-errors="fieldErrors"
        :suggestions-for="suggestionsFor"
        :open-media-picker="openMediaPicker"
        :disabled="disabled"
        :switches="switches"
        @update="update"
      />
    </BaseDisclosure>
  </div>
</template>

<style scoped>
.base-schema-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
