import type { Component } from 'vue';
import type { JsonValue } from '../../../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../../../shared/messages.ts';
import type { SchemaFieldDescriptor } from './schema-fields.ts';
import type { SchemaSuggestion } from './schema-suggestion.ts';

export interface SchemaFieldControlProps {
  field: SchemaFieldDescriptor;
  value: JsonValue | undefined;
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
