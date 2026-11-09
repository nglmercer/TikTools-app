<script setup lang="ts">
import type { JsonObject, JsonValue } from '../../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../../shared/messages.ts';
import type { SchemaFieldDescriptor } from './lib/schema-fields.ts';
import type { SchemaSuggestion } from './lib/schema-suggestion.ts';
import SchemaFieldControl from './SchemaFieldControl.vue';

defineProps<{
  fields: readonly SchemaFieldDescriptor[];
  values: JsonObject;
  formId: string;
  fieldErrors: Readonly<Record<string, string>>;
  suggestionsFor?: (key: string) => readonly SchemaSuggestion[];
  openMediaPicker?: OpenMediaPicker;
  disabled?: boolean;
  switches?: boolean;
}>();

const emit = defineEmits<{ update: [field: SchemaFieldDescriptor, value: JsonValue] }>();

function controlId(formId: string, field: SchemaFieldDescriptor): string {
  return `${formId}-${field.key.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
}
</script>

<template>
  <div class="schema-field-rows">
    <template
      v-for="field in fields"
      :key="field.key"
    >
      <label
        v-if="field.kind !== 'boolean'"
        class="schema-field-rows__label"
        :for="controlId(formId, field)"
      >{{ field.label }}</label>
      <span v-else />
      <div class="schema-field-rows__control">
        <SchemaFieldControl
          :field="field"
          :value="values[field.key]"
          :control-id="controlId(formId, field)"
          :invalid="Boolean(fieldErrors[field.key])"
          :disabled="disabled"
          :switches="switches"
          :suggestions="suggestionsFor?.(field.key)"
          :open-media-picker="openMediaPicker"
          @update="(value) => emit('update', field, value)"
        />
        <p
          v-if="fieldErrors[field.key]"
          class="schema-field-rows__error"
          role="alert"
        >
          {{ fieldErrors[field.key] }}
        </p>
        <p
          v-else-if="field.hint"
          class="schema-field-rows__hint"
        >
          {{ field.hint }}
        </p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.schema-field-rows {
  display: grid;
  grid-template-columns: minmax(120px, max-content) minmax(0, 1fr);
  gap: var(--space-4) var(--space-5);
  align-items: start;
}

.schema-field-rows__label {
  justify-self: end;
  padding-top: var(--space-2);
  color: var(--color-text);
  text-align: right;
}

.schema-field-rows__control {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.schema-field-rows__hint {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}

.schema-field-rows__error {
  margin: 0;
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}
</style>
