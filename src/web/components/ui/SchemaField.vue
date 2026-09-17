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
import { TemplateField } from '../node-editor/TemplateField.vue';
import { getFetchUrlTemplates } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import type { Locale } from '../../i18n.ts';
import type { JsonObject, JsonValue } from '../../../automation/types.ts';
import { isSecretField } from '../../../automation/plugins/declarative.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { localized, toDisplayValue, formatJson, type FieldOption } from './schema-form-helpers.ts';
import { KeyValueEditor } from './KeyValueEditor.vue';

export function SchemaField({ locale, name, schema, hint, value, onChange, templateSuggestions, fieldOptions, onOpenMediaPicker }: {
  locale: Locale;
  name: string;
  schema: JsonObject;
  hint?: JsonObject;
  value: JsonValue | undefined;
  onChange: (value: JsonValue) => void;
  templateSuggestions: AutocompleteItem[];
  fieldOptions?: FieldOption[];
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
          autoComplete="current-password"
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
    // Only hinted options can carry icons; when any does, the whole list
    // renders as an IconSelect because native <option> cannot draw SVGs.
    const useIcons = options === hintedEntries && hintedEntries.some((entry) => entry.icon !== undefined);
    const iconOptions: IconSelectOption[] | undefined = useIcons
      ? hintedEntries.map((entry) => ({
        value: entry.value,
        label: entry.label,
        hint: entry.hint,
        icon: entry.icon ? <Icon name={entry.icon} size={14} /> : undefined,
      }))
      : undefined;
    return (
      <SelectField
        name={name}
        label={label}
        hintText={hintText}
        template={template}
        value={displayValue}
        options={options as SelectOption[]}
        iconOptions={iconOptions}
        onChange={onChange}
      />
    );
  }

  if (kind === 'textarea' || kind === 'code' || schema.type === 'array' || schema.format === 'json') {
    // Templated textareas (Body…) get the code editor: line numbers, JSON
    // highlight, `{{ }}` pills and variable autocomplete.
    if (hasAutocomplete && schema.type !== 'array' && kind !== 'code') {
      const json = schema.format === 'json';
      const editorValue = json && typeof value === 'string' ? value : displayValue;
      return (
        <div class="plg-field">
          <div class="plg-label-row">
            <label class="plg-label">{label}</label>
            {hintText ? <InfoTip text={hintText} position="right" /> : null}
          </div>
          <CodeEditor
            locale={locale}
            language={json ? 'json' : 'text'}
            name={name}
            value={editorValue}
            onValueChange={onChange}
            suggestions={templateSuggestions}
            filename={json ? `${name}.json` : undefined}
            mime={json ? 'application/json' : undefined}
            rows={6}
            ariaLabel={label}
            onFormat={json ? () => {
              const formatted = formatJsonText(editorValue);
              if (formatted !== null && formatted !== editorValue) onChange(formatted);
            } : undefined}
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
          onValueChange={(next) => onChange(next === null ? '' : next)}
          min={typeof schema.minimum === 'number' ? schema.minimum : typeof schema.min === 'number' ? schema.min : undefined}
          max={typeof schema.maximum === 'number' ? schema.maximum : typeof schema.max === 'number' ? schema.max : undefined}
          step={typeof schema.multipleOf === 'number' ? schema.multipleOf : undefined}
        />
      </div>
    );
  }

  // Text: templated (or with pushed suggestions) → autocomplete input.
  // URL-like fields stay quiet while typing a hostname: bare words match URL
  // presets in the dropdown, variables only inside `{{ }}` or via Ctrl+Space.
  if (hasAutocomplete) {
    const isUrlField = /url|link|endpoint|webhook/i.test(name);
    return (
      <div class="plg-field">
        <TemplateField
          locale={locale}
          name={name}
          value={displayValue}
          onValueChange={onChange}
          suggestions={templateSuggestions}
          ariaLabel={label}
          label={label}
          hint={hintText || undefined}
          bareWordTrigger={!isUrlField}
          urlPresets={isUrlField ? getFetchUrlTemplates() : undefined}
        />
      </div>
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
        leadingIcon={fieldIconName ? <Icon name={fieldIconName} size={14} /> : undefined}
      />
    </div>
  );
}
/** Select with floating label + tooltips on every option (`title`). Choices with icons use IconSelect. */
function SelectField({
  name,
  label,
  hintText,
  value,
  options,
  iconOptions,
  onChange,
}: {
  name: string;
  label: string;
  hintText: string;
  template?: boolean;
  value: string;
  options: SelectOption[];
  iconOptions?: IconSelectOption[];
  onChange: (value: JsonValue) => void;
}) {
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
          onChange={(next) => onChange(next)}
        />
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
        onValueChange={(next) => onChange(next)}
      />
    </div>
  );
}

export default SchemaField;
</script>
