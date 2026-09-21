import type { Locale } from '../../i18n.ts';
import type { JsonObject } from '../../../automation/types.ts';
import { readIconName, type IconName } from '../icons/index.ts';
import { localized, type FieldOption } from './schema-form-helpers.ts';

export type SchemaSelectOption = {
  value: string;
  label: string;
  hint?: string;
  icon?: IconName;
};

export type FieldOptionsSource = 'schema' | 'dynamic' | 'hinted' | 'none';

/**
 * Resolves the select options for one schema field. Precedence is static
 * schema enums first, then dynamic (optionsFrom) lists, then hinted lists.
 * An empty result means the field is not a select at all.
 */
export function resolveFieldOptions(
  schema: JsonObject,
  hint: JsonObject | undefined,
  fieldOptions: FieldOption[] | undefined,
  locale: Locale,
): { options: SchemaSelectOption[]; source: FieldOptionsSource } {
  const schemaOptions = Array.isArray(schema.enum)
    ? schema.enum.filter((entry): entry is string => typeof entry === 'string').map((value) => ({ value, label: value }))
    : [];
  const hintedEntries: SchemaSelectOption[] = Array.isArray(hint?.options)
    ? hint.options.filter((entry): entry is JsonObject => Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry)).map((entry) => ({
      value: typeof entry.value === 'string' ? entry.value : '',
      label: localized(entry.label, locale) || (typeof entry.value === 'string' ? entry.value : ''),
      hint: localized(entry.hint, locale) || undefined,
      icon: readIconName(entry.icon),
    }))
    : [];
  const dynamicOptions = Array.isArray(fieldOptions) ? fieldOptions.filter((entry) => entry && typeof entry.value === 'string') : [];
  if (schemaOptions.length > 0) return { options: schemaOptions, source: 'schema' };
  if (dynamicOptions.length > 0) return { options: dynamicOptions, source: 'dynamic' };
  if (hintedEntries.length > 0) return { options: hintedEntries, source: 'hinted' };
  return { options: [], source: 'none' };
}
