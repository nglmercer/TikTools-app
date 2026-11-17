import type { Component } from 'vue';
import type { JsonArray, JsonObject } from '../../../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../../../shared/messages.ts';
import type { SchemaFieldDescriptor } from './schema-fields.ts';
import type { SchemaSuggestion } from './schema-suggestion.ts';

/**
 * Same shape as `JsonValue | undefined`, but with `string` listed first.
 *
 * Vue casts `''` to `true` for a prop when Boolean appears before String in
 * its runtime type list. `JsonValue` starts with `null | boolean`, so a text
 * field whose default is `""` arrived at `SchemaFieldControl` as boolean
 * `true` and could not be edited. Keeping `string` first is enough to stop
 * that cast while still accepting real booleans for boolean fields.
 */
type FieldValue = string | number | boolean | JsonObject | JsonArray | null | undefined;

export interface SchemaFieldControlProps {
  field: SchemaFieldDescriptor;
  value: FieldValue;
  controlId: string;
  invalid?: boolean;
  disabled?: boolean;
  switches?: boolean;
  suggestions?: readonly SchemaSuggestion[];
  openMediaPicker?: OpenMediaPicker;
  /** Dense layout: hint text is hidden and numbers use a −/+ stepper. */
  compact?: boolean;
  /** Optional leading icon rendered inside the control. */
  icon?: Component;
}
