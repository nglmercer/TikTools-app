<script setup lang="ts">
import type { JsonObject, JsonValue } from '../../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../../shared/messages.ts';
import type { Component } from 'vue';
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
  compact?: boolean;
  fieldIcons?: Readonly<Record<string, Component>>;
}>();

const emit = defineEmits<{ update: [field: SchemaFieldDescriptor, value: JsonValue] }>();

function controlId(formId: string, field: SchemaFieldDescriptor): string {
  return `${formId}-${field.key.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
}
</script>

<template>
  <div
    class="schema-field-rows"
    :class="{ 'schema-field-rows--compact': compact }"
  >
    <template
      v-for="field in fields"
      :key="field.key"
    >
      <label
        v-if="!compact && field.kind !== 'boolean'"
        class="schema-field-rows__label"
        :for="controlId(formId, field)"
      >{{ field.label }}</label>
      <span v-else-if="!compact" />
      <div
        class="schema-field-rows__control"
        :class="{ 'schema-field-rows__control--compact': compact }"
      >
        <label
          v-if="compact && field.kind !== 'boolean'"
          class="schema-field-rows__sr"
          :for="controlId(formId, field)"
        >{{ field.label }}</label>
        <SchemaFieldControl
          :field="field"
          :value="values[field.key]"
          :control-id="controlId(formId, field)"
          :invalid="Boolean(fieldErrors[field.key])"
          :disabled="disabled"
          :switches="switches"
          :compact="compact"
          :icon="compact ? fieldIcons?.[field.key] : undefined"
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
          v-else-if="field.hint && !compact"
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

.schema-field-rows--compact {
  grid-template-columns: minmax(0, 1fr);
  gap: var(--space-3);
}

.schema-field-rows--compact .schema-field-rows__control {
  gap: var(--space-1);
}

/* Keep the field name for assistive technology without repeating it visually. */
.schema-field-rows__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  padding: 0;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
  border: 0;
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
