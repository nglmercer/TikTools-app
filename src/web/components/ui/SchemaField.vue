<script lang="tsx">
import { Icon, readIconName, type IconName } from '../icons/index.ts';
import { IconSelect, type IconSelectOption } from './IconSelect.vue';
import { InfoTip } from './InfoTip.vue';
import { MediaField } from './MediaField.vue';
import { NumberInput } from './NumberInput.vue';
import { Select } from './Select.vue';
import { TextInput } from './TextInput.vue';
import type { SelectOption } from './controls.ts';
import { DatePicker, TimePicker } from './DatePicker.vue';
import { PasswordInput } from './PasswordInput.vue';
import { ColorPicker } from './ColorPicker.vue';
import { Textarea } from './Textarea.vue';
import { Range } from './Range.vue';
import { Rating } from './Rating.vue';
import { TagsInput } from './TagsInput.vue';
import { MultiSelect } from './MultiSelect.vue';
import { CodeEditor, formatJsonText } from './CodeEditor.vue';
import { TemplateField } from './fields/TemplateField.vue';
import { TextField } from './fields/TextField.vue';
import { getFetchUrlTemplates } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import type { Locale } from '../../i18n.ts';
import type { JsonObject, JsonValue } from '../../../automation/types.ts';
import { isSecretField } from '../../../automation/plugins/declarative.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { defineVueComponent } from '../../vue/component.ts';
import { localized, toDisplayValue, resolveSelectDisplayValue, formatJson, type FieldOption } from './schema-form-helpers.ts';
import { KeyValueEditor } from './KeyValueEditor.vue';

export type SchemaFieldProps = {
  locale: Locale;
  name: string;
  schema: JsonObject;
  hint?: JsonObject;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  templateSuggestions: AutocompleteItem[];
  fieldOptions?: FieldOption[];
  error?: string;
  onOpenMediaPicker?: OpenMediaPicker;
};

/**
 * Declared component (not a plain render function): props are registered at
 * runtime and `inheritAttrs` is disabled, so a listener-looking `onChange`
 * prop can never fall through onto the rendered native tree and a bubbled
 * DOM `change` Event can never overwrite settings state.
 */
export const SchemaField = defineVueComponent<SchemaFieldProps>(
  ['locale', 'name', 'schema', 'hint', 'value', 'onChange', 'templateSuggestions', 'fieldOptions', 'error', 'onOpenMediaPicker'],
  (props) => {
    return () => {
      const {
        locale,
        name,
        schema,
        hint,
        value,
        onChange,
        templateSuggestions,
        fieldOptions,
        error,
        onOpenMediaPicker,
      } = props;
      return renderSchemaField({
        locale,
        name,
        schema,
        hint,
        value,
        onChange,
        templateSuggestions,
        fieldOptions,
        error,
        onOpenMediaPicker,
      });
    };
  },
);

function renderSchemaField({ locale, name, schema, hint, value, onChange, templateSuggestions, fieldOptions, error, onOpenMediaPicker }: {
  locale: Locale;
  name: string;
  schema: JsonObject;
  hint?: JsonObject;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  templateSuggestions: AutocompleteItem[];
  fieldOptions?: FieldOption[];
  error?: string;
  onOpenMediaPicker?: OpenMediaPicker;
}) {
  const label = localized(schema.title, locale) || name;
  const description = typeof schema.description === 'string' ? schema.description : localized(schema.description as JsonValue, locale);
  const hintText = localized(hint?.hint, locale) || description;
  const fieldIconName = readIconName(hint?.icon);
  const kind = typeof hint?.kind === 'string' ? hint.kind : schema.format === 'code' ? 'code' : schema.type;
  const template = hint?.template === true;
  const displayValue = toDisplayValue(value, schema.type);
  const hasAutocomplete = template || templateSuggestions.length > 0;

  if (kind === 'media') {
    return (
      <MediaField
        locale={locale}
        name={name}
        label={label}
        hint={hintText || undefined}
        value={value}
        onValueChange={onChange}
        onOpenMediaPicker={onOpenMediaPicker}
        mode={hint?.mode === 'directory' ? 'directory' : 'file'}
        kind={hint?.mediaKind === 'video' || hint?.mediaKind === 'image' || hint?.mediaKind === 'other' ? hint.mediaKind : 'audio'}
        extensions={Array.isArray(hint?.extensions) ? hint.extensions.filter((entry): entry is string => typeof entry === 'string') : undefined}
      />
    );
  }

  if (kind === 'boolean' || schema.type === 'boolean') {
    const checked = value === true || value === 'true';
    const controlId = `schema-${name.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
    return (
      <div class="plg-field">
        <div class="plg-switch-row">
          <label class={`plg-switch plg-switch--field${checked ? ' is-on' : ''}`} for={controlId}>
            <input
              id={controlId}
              class="plg-switch__input"
              type="checkbox"
              name={name}
              checked={checked}
              aria-label={label}
              onChange={(event) => onChange((event.currentTarget as HTMLInputElement).checked)}
            />
            <span class="plg-switch__track"><span class="plg-switch__thumb" /></span>
          </label>
          <label class="plg-label" for={controlId}>{label}</label>
          {hintText ? <InfoTip text={hintText} position="right" /> : null}
        </div>
      </div>
    );
  }

  if (kind === 'keyvalue' || (schema.type === 'object' && schema.additionalProperties !== undefined)) {
    return (
      <KeyValueEditor
        locale={locale}
        label={label}
        hintText={hintText}
        entries={value && typeof value === 'object' && !Array.isArray(value) ? value as JsonObject : {}}
        suggestions={templateSuggestions}
        onChange={onChange}
      />
    );
  }

  if (schema.format === 'date' || hint?.kind === 'date') {
    return (
      <div class="plg-field">
        <DatePicker
          name={name}
          label={label}
          hint={hintText || undefined}
          value={typeof value === 'string' ? value : ''}
          onValueChange={onChange}
          min={typeof schema.minimum === 'string' ? schema.minimum : typeof schema.min === 'string' ? schema.min : undefined}
          max={typeof schema.maximum === 'string' ? schema.maximum : typeof schema.max === 'string' ? schema.max : undefined}
        />
      </div>
    );
  }

  if (schema.format === 'time' || hint?.kind === 'time') {
    return (
      <div class="plg-field">
        <TimePicker
          name={name}
          label={label}
          hint={hintText || undefined}
          value={typeof value === 'string' ? value : ''}
          onValueChange={onChange}
        />
      </div>
    );
  }

  if (schema.format === 'color' || hint?.kind === 'color') {
    const color = typeof value === 'string' && value ? value : '#000000';
    return (
      <div class="plg-field">
        <ColorPicker
          name={name}
          label={label}
          hint={hintText || undefined}
          value={color}
          onValueChange={onChange}
        />
      </div>
    );
  }

  if (schema.format === 'password' || hint?.kind === 'password' || isSecretField(schema, hint)) {
    return (
      <div class="plg-field">
        <PasswordInput
          name={name}
          label={label}
          hint={hintText || undefined}
          value={typeof value === 'string' ? value : ''}
          onValueChange={onChange}
          error={error}
          autoComplete="current-password"
          locale={locale}
        />
      </div>
    );
  }

  if (hint?.kind === 'range' && (schema.type === 'number' || schema.type === 'integer')) {
    const numeric = typeof value === 'number' ? value : Number(value ?? 0);
    return (
      <div class="plg-field">
        <Range
          name={name}
          label={label}
          hint={hintText || undefined}
          value={Number.isFinite(numeric) ? numeric : 0}
          onValueChange={onChange}
          min={typeof schema.minimum === 'number' ? schema.minimum : typeof schema.min === 'number' ? schema.min : 0}
          max={typeof schema.maximum === 'number' ? schema.maximum : typeof schema.max === 'number' ? schema.max : 100}
          step={typeof schema.multipleOf === 'number' ? schema.multipleOf : 1}
        />
      </div>
    );
  }

  if (hint?.kind === 'rating') {
    const numeric = typeof value === 'number' ? value : Number(value ?? 0);
    return (
      <div class="plg-field">
        <Rating
          name={name}
          label={label}
          hint={hintText || undefined}
          value={Number.isFinite(numeric) ? numeric : 0}
          onValueChange={onChange}
          max={typeof schema.maximum === 'number' ? schema.maximum : 5}
        />
      </div>
    );
  }

  if (schema.type === 'array' && schema.items && typeof schema.items === 'object' && !Array.isArray(schema.items) && Array.isArray((schema.items as JsonObject).enum)) {
    const allowed = ((schema.items as JsonObject).enum as unknown[]).filter((entry): entry is string => typeof entry === 'string');
    const selected = Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];
    return (
      <div class="plg-field">
        <MultiSelect
          name={name}
          label={label}
          hint={hintText || undefined}
          value={selected}
          options={allowed.map((entry) => ({ value: entry, label: entry }))}
          onValueChange={onChange}
        />
      </div>
    );
  }

  if (schema.type === 'array' && (hint?.kind === 'tags' || (schema.items && typeof schema.items === 'object' && (schema.items as JsonObject).type === 'string'))) {
    const selected = Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];
    return (
      <div class="plg-field">
        <TagsInput
          name={name}
          label={label}
          hint={hintText || undefined}
          value={selected}
          onValueChange={onChange}
        />
      </div>
    );
  }

  if ((kind === 'textarea' || schema.format === 'multiline') && !hasAutocomplete && schema.type === 'string') {
    return (
      <div class="plg-field">
        <Textarea
          name={name}
          label={label}
          hint={hintText || undefined}
          value={typeof value === 'string' ? value : ''}
          onValueChange={onChange}
          rows={6}
          maxLength={typeof schema.maxLength === 'number' ? schema.maxLength : undefined}
          showCount={typeof schema.maxLength === 'number'}
        />
      </div>
    );
  }

  const schemaOptions = Array.isArray(schema.enum) ? schema.enum.filter((entry): entry is string => typeof entry === 'string').map((value) => ({ value, label: value })) : [];
  const hintedEntries: Array<{ value: string; label: string; hint?: string; icon?: IconName }> = Array.isArray(hint?.options)
    ? hint.options.filter((entry): entry is JsonObject => Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry)).map((entry) => ({
      value: typeof entry.value === 'string' ? entry.value : '',
      label: localized(entry.label, locale) || (typeof entry.value === 'string' ? entry.value : ''),
      hint: localized(entry.hint, locale) || undefined,
      icon: readIconName(entry.icon),
    }))
    : [];
  const dynamicOptions = Array.isArray(fieldOptions) ? fieldOptions.filter((entry) => entry && typeof entry.value === 'string') : [];
  const options = schemaOptions.length > 0 ? schemaOptions : dynamicOptions.length > 0 ? dynamicOptions : hintedEntries;
  if (options.length > 0) {
    // Defaults are applied deliberately at the settings/form state boundary
    // (`withSchemaDefaults`, plus the host overlay), not here. This render
    // step only fills a genuinely absent value with the schema default; an
    // invalid stored value (e.g. "xx" vs enum [en, es]) stays observable
    // instead of silently masquerading as the default.
    const optionValues = new Set(options.map((entry) => entry.value));
    const schemaDefault = typeof schema.default === 'string' ? schema.default : undefined;
    const effectiveValue = resolveSelectDisplayValue(value, displayValue, schemaDefault, optionValues);
    // Dynamic (optionsFrom) lists and icon-carrying hinted lists render as an
    // IconSelect; static lists use the native Select. Native <option> cannot
    // draw SVGs, which is why icon lists need the custom control.
    const isDynamic = options === dynamicOptions;
    const useIcons = isDynamic || (options === hintedEntries && hintedEntries.some((entry) => entry.icon !== undefined));
    const iconOptions: IconSelectOption[] | undefined = !useIcons
      ? undefined
      : isDynamic
        ? dynamicOptions.map((entry) => ({ value: entry.value, label: entry.label }))
        : hintedEntries.map((entry) => ({
          value: entry.value,
          label: entry.label,
          hint: entry.hint,
          icon: entry.icon ? <Icon name={entry.icon} size={14} /> : undefined,
        }));
    return renderSelectField({
      name,
      label,
      hintText,
      value: effectiveValue,
      options: options as SelectOption[],
      iconOptions,
      error,
      onValueChange: onChange,
    });
  }

  if (kind === 'textarea' || kind === 'code' || schema.type === 'array' || schema.format === 'json') {
    // Templated plain-text textareas (TTS text…) get a multiline
    // TemplateField; JSON keeps the code editor (highlight + validation).
    if (hasAutocomplete && schema.type !== 'array' && kind !== 'code') {
      if (schema.format !== 'json') {
        return (
          <TemplateField
            locale={locale}
            name={name}
            label={label}
            hint={hintText || undefined}
            value={displayValue}
            onValueChange={onChange}
            suggestions={templateSuggestions}
            multiline
            rows={6}
            error={error}
            placeholder={typeof hint?.placeholder === 'string' ? hint.placeholder : undefined}
          />
        );
      }
      const editorValue = typeof value === 'string' ? value : displayValue;
      return (
        <div class="plg-field">
          <div class="plg-label-row">
            <label class="plg-label">{label}</label>
            {hintText ? <InfoTip text={hintText} position="right" /> : null}
          </div>
          <CodeEditor
            locale={locale}
            language="json"
            name={name}
            value={editorValue}
            onValueChange={onChange}
            suggestions={templateSuggestions}
            filename={`${name}.json`}
            mime="application/json"
            rows={6}
            ariaLabel={label}
            onFormat={() => {
              const formatted = formatJsonText(editorValue);
              if (formatted !== null && formatted !== editorValue) onChange(formatted);
            }}
          />
        </div>
      );
    }
    const text = schema.type === 'array' ? formatJson(value) : schema.format === 'json' && typeof value === 'string' ? value : displayValue;
    const filled = text.trim().length > 0;
    return (
      <div class="plg-field">
        <div class={`plg-float ${filled ? 'is-filled' : ''}`}>
          <div class="plg-float__control plg-float__control--textarea">
            <textarea
              name={name}
              rows={kind === 'code' ? 16 : 6}
              spellcheck={false}
              value={text}
              placeholder=" "
              aria-label={label}
              onInput={(event) => {
                const next = (event.currentTarget as HTMLTextAreaElement).value;
                if (schema.type === 'array') {
                  try { onChange(JSON.parse(next) as JsonValue); } catch { onChange(next); }
                } else onChange(next);
              }}
            />
            <label class="plg-float__label">
              {label}
              {hintText ? <InfoTip text={hintText} position="right" /> : null}
            </label>
          </div>
        </div>
      </div>
    );
  }

  if (kind === 'number' || schema.type === 'number' || schema.type === 'integer') {
    const numeric = displayValue.trim() === '' ? null : Number(displayValue);
    return (
      <div class="plg-field">
        <NumberInput
          name={name}
          label={label}
          hint={hintText || undefined}
          value={numeric !== null && Number.isFinite(numeric) ? numeric : null}
          error={error}
          onValueChange={(next) => onChange(next === null ? '' : next)}
          min={typeof schema.minimum === 'number' ? schema.minimum : typeof schema.min === 'number' ? schema.min : undefined}
          max={typeof schema.maximum === 'number' ? schema.maximum : typeof schema.max === 'number' ? schema.max : undefined}
          step={typeof schema.multipleOf === 'number' ? schema.multipleOf : undefined}
        />
      </div>
    );
  }

  // Text: templated (or with pushed suggestions) → TemplateField.
  // Plain URL fields get preset shortcuts but NEVER event variables;
  // templated URL fields add presets beside `{{ }}` variables.
  if (hasAutocomplete) {
    const isUrlField = /url|link|endpoint|webhook/i.test(name) || schema.format === 'uri';
    const placeholder = typeof hint?.placeholder === 'string' ? hint.placeholder : undefined;
    if (isUrlField && !template) {
      return (
        <TextField
          locale={locale}
          name={name}
          label={label}
          hint={hintText || undefined}
          value={displayValue}
          onValueChange={onChange}
          error={error}
          leadingIcon="globe"
          presets={getFetchUrlTemplates()}
          placeholder={placeholder}
        />
      );
    }
    return (
      <TemplateField
        locale={locale}
        name={name}
        label={label}
        hint={hintText || undefined}
        value={displayValue}
        onValueChange={onChange}
        suggestions={templateSuggestions}
        scope={isUrlField ? 'http-url' : 'generic'}
        presets={isUrlField ? getFetchUrlTemplates() : undefined}
        error={error}
        placeholder={placeholder}
      />
    );
  }

  return (
    <div class="plg-field">
      <TextInput
        name={name}
        label={label}
        hint={hintText || undefined}
        value={displayValue}
        onValueChange={onChange}
        error={error}
        leadingIcon={fieldIconName ? <Icon name={fieldIconName} size={14} /> : undefined}
      />
    </div>
  );
}
/** Select with floating label + tooltips on every option (`title`). Choices with icons use IconSelect. Called directly (not rendered as a component vnode), so Vue can never perform listener fallthrough on the callback. */
type SelectRenderArgs = {
  name: string;
  label: string;
  hintText: string;
  value: string;
  options: SelectOption[];
  iconOptions?: IconSelectOption[];
  error?: string;
  onValueChange: (value: JsonValue) => void;
};

function renderSelectField({
  name,
  label,
  hintText,
  value,
  options,
  iconOptions,
  error,
  onValueChange,
}: SelectRenderArgs) {
  if (iconOptions) {
    return (
      <div class="plg-field">
        <div class="plg-label-row">
          <label class="plg-label">{label}</label>
          {hintText ? <InfoTip text={hintText} position="right" /> : null}
        </div>
        <IconSelect
          ariaLabel={label}
          value={value}
          options={iconOptions}
          onChange={(next) => onValueChange(next)}
          invalid={Boolean(error)}
        />
        {error ? <span class="field-message field-message--error">{error}</span> : null}
      </div>
    );
  }
  return (
    <div class="plg-field">
      <Select
        name={name}
        label={label}
        hint={hintText || undefined}
        value={value}
        options={options}
        error={error}
        onValueChange={(next) => onValueChange(next)}
      />
    </div>
  );
}

export default SchemaField;
</script>
