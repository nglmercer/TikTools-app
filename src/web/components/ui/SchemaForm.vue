<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { readFormValues, type FormSchema } from './control-events.ts';

import type { AutomationEvent, AutomationEventType, JsonObject, JsonValue } from '../../../automation/types.ts';
import type { ActionTypeDefinition } from '../../../automation/behavior/types.ts';
import { TemplateField } from '../node-editor/TemplateField.vue';
import { getFetchUrlTemplates, getTemplateSuggestions, type TemplateSuggestionScope } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { resolveAutocompleteSources as mergeAutocompleteSources, suggestionsFromObject } from '../autocomplete/index.ts';
import { AdvancedSection } from './FieldPanels.vue';
import { CodeEditor, formatJsonText } from './CodeEditor.vue';
import { InfoTip } from './InfoTip.vue';
import { i18nText, t, type Locale } from '../../i18n.ts';
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
import type { OpenMediaPicker } from '../../../shared/messages.ts';

export type FieldOption = { value: string; label: string };

export type SchemaFormProps = {
  locale: Locale;
  schema: JsonObject;
  uiHints?: JsonObject;
  value: JsonObject;
  onChange: (value: JsonObject) => void;
  /** Additional suggestions merged with the host-provided context. */
  templateSuggestions?: AutocompleteItem[];
  /** Any object pushed as autocomplete (live event, custom schema sample…). */
  suggestionContext?: JsonValue | AutomationEvent;
  /** Per-field scopes, e.g. `{ url: 'http-url', body: 'http-data' }`. */
  suggestionScopes?: Partial<Record<string, TemplateSuggestionScope>>;
  /** Trigger used to pick trigger-specific paths when no explicit list given. */
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  /** Dynamic per-field options fetched on demand (voices, devices, …). */
  fieldOptions?: Record<string, FieldOption[]>;
  /** Opens a host-owned native media dialog and returns a path reference. */
  onOpenMediaPicker?: OpenMediaPicker;
};

export type SchemaFormHandle = {
  getValues: () => Record<string, unknown>;
};

/** Default scope per field name so Call-URL-like forms work with zero config. */
function defaultScopeFor(name: string, template: boolean): TemplateSuggestionScope {
  const key = name.toLowerCase();
  if (key.includes('url') || key.includes('link') || key.includes('endpoint') || key.includes('webhook')) return 'http-url';
  if (key.includes('uniqueid') || key.includes('viewer') || key.includes('user') || key === 'key' || key === 'type') return 'identity';
  if (key.includes('file') || key.includes('sound') || key.includes('audio') || key.includes('path')) return 'sound-file';
  if (key.includes('comment') || key.includes('message') || key.includes('text')) return 'text';
  if (key.includes('leftpath') || key.includes('path')) return 'compare';
  return template ? 'http-data' : 'message';
}

/**
 * Shared suggestion resolver so custom editors (e.g. the fetch layout) offer
 * exactly the same variables as the generic form: trigger scope plus any
 * object pushed via `suggestionContext`.
 */
export function resolveAutocompleteSources(args: {
  locale: Locale;
  suggestionContext?: JsonValue | AutomationEvent;
  suggestionScopes?: Partial<Record<string, TemplateSuggestionScope>>;
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  templateSuggestions?: AutocompleteItem[];
}): (name: string, template: boolean) => AutocompleteItem[] {
  return (name: string, template: boolean): AutocompleteItem[] => {
    const {
      locale,
      suggestionContext,
      suggestionScopes = {},
      eventType,
      lastEvent,
      templateSuggestions = [],
    } = args;
    let contextItems: AutocompleteItem[] = [];
    if (suggestionContext !== undefined) {
      const root = suggestionContext as JsonValue;
      // `AutomationEvent` arrives as `{ type, user, data… }` — expose as `event.*`.
      if (root !== null && typeof root === 'object' && !Array.isArray(root) && 'type' in (root as JsonObject) && !('event' in (root as JsonObject))) {
        contextItems = suggestionsFromObject({ event: root } as unknown as JsonValue, '', { maxItems: 80 });
      } else {
        contextItems = suggestionsFromObject(root, '', { maxItems: 80 });
      }
    }
    const scope = suggestionScopes[name] ?? defaultScopeFor(name, template);
    const scoped = getTemplateSuggestions(eventType, locale, lastEvent, scope, undefined);
    return mergeAutocompleteSources(scoped, contextItems, templateSuggestions);
  };
}

/**
 * Small, deliberately bounded JSON Schema renderer. It renders data, never
 * code: plugin packages can describe forms but cannot inject DOM or Vue code.
 *
 * Every field gets a tooltip (InfoTip) when it has a hint, inline `{{ }}`
 * highlight, and autocomplete when it can use `{{ event.* }}` — from the
 * trigger scope plus any object pushed via `suggestionContext`.
 */
export const SchemaForm = defineVueComponent<SchemaFormProps>(
  [
    'locale',
    'schema',
    'uiHints',
    'value',
    'onChange',
    'templateSuggestions',
    'suggestionContext',
    'suggestionScopes',
    'eventType',
    'lastEvent',
    'fieldOptions',
    'onOpenMediaPicker',
  ],
  (props, context) => {
  const formRef = ref<HTMLDivElement | null>(null);
  const properties = computed(() => objectProperties(props.schema.properties));
  const controlSchema = computed(() => formSchemaFromJsonSchema(props.schema));
  context.expose({
    getValues: () => formRef.value ? readFormValues(formRef.value, controlSchema.value) : {},
  } satisfies SchemaFormHandle);

  return () => {
  const templateSuggestions = props.templateSuggestions ?? [];
  const suggestionScopes = props.suggestionScopes ?? {};
  const fieldOptions = props.fieldOptions ?? {};
  const hints = objectProperties(props.uiHints?.fields);
  const visible = Object.entries(properties.value).filter(([key]) => applies(hints[key]?.showIf, props.value));
  const basic = visible.filter(([key]) => hints[key]?.advanced !== true);
  const advanced = visible.filter(([key]) => hints[key]?.advanced === true);
  const update = (key: string, next: JsonValue): void => props.onChange({ ...props.value, [key]: next });

  const suggestionsFor = resolveAutocompleteSources({
    locale: props.locale,
    suggestionContext: props.suggestionContext,
    suggestionScopes,
    eventType: props.eventType,
    lastEvent: props.lastEvent,
    templateSuggestions,
  });

  return (
      <div ref={formRef} class="plg-form__schema" data-tiktools-form="schema">
      {basic.map(([key, field]) => (
        <SchemaField
          key={key}
          locale={props.locale}
          name={key}
          schema={field}
          hint={hints[key]}
          value={props.value[key]}
          onChange={(next) => update(key, next)}
          templateSuggestions={suggestionsFor(key, (hints[key]?.template as boolean) === true)}
          fieldOptions={fieldOptions[key]}
          onOpenMediaPicker={props.onOpenMediaPicker}
        />
      ))}
      {advanced.length > 0 && (
        <AdvancedSection
          title={t(props.locale, 'advancedOptions')}
          hint={t(props.locale, 'advancedHttpHint')}
          count={advanced.length}
        >
          {advanced.map(([key, field]) => (
            <SchemaField
              key={key}
              locale={props.locale}
              name={key}
              schema={field}
              hint={hints[key]}
              value={props.value[key]}
              onChange={(next) => update(key, next)}
              templateSuggestions={suggestionsFor(key, (hints[key]?.template as boolean) === true)}
              fieldOptions={fieldOptions[key]}
              onOpenMediaPicker={props.onOpenMediaPicker}
            />
          ))}
        </AdvancedSection>
      )}
    </div>
  );
  };
  },
);

export function schemaForAction(type: ActionTypeDefinition): { schema: JsonObject; uiHints?: JsonObject } {
  return {
    schema: type.configSchema ?? schemaFromFields(type),
    uiHints: type.uiHints ?? hintsFromFields(type),
  };
}

function formSchemaFromJsonSchema(schema: JsonObject): FormSchema {
  const properties = objectProperties(schema.properties);
  const entries = Object.entries(properties).map(([name, field]) => {
    const type = field.format === 'json'
      ? 'json'
      : field.type === 'boolean'
        ? 'boolean'
        : field.type === 'integer'
          ? 'integer'
          : field.type === 'number'
            ? 'number'
            : 'string';
    return [name, { type, defaultValue: field.default }] as const;
  });
  return Object.fromEntries(entries) as FormSchema;
}

function SchemaField({ locale, name, schema, hint, value, onChange, templateSuggestions, fieldOptions, onOpenMediaPicker }: {
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
          <label class={`plg-switch plg-switch--field${checked ? ' is-on' : ''}`} for={controlId} data-tooltip={hintText || undefined} data-tooltip-pos="right" data-tooltip-wide={hintText ? '' : undefined}>
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

  if (schema.format === 'password' || hint?.kind === 'password') {
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
  const hintedOptions = Array.isArray(hint?.options)
    ? hint.options.filter((entry): entry is JsonObject => Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry)).map((entry) => ({
      value: typeof entry.value === 'string' ? entry.value : '',
      label: localized(entry.label, locale) || (typeof entry.value === 'string' ? entry.value : ''),
      hint: localized((entry as JsonObject).hint, locale) || undefined,
    }))
    : [];
  const dynamicOptions = Array.isArray(fieldOptions) ? fieldOptions.filter((entry) => entry && typeof entry.value === 'string') : [];
  const options = schemaOptions.length > 0 ? schemaOptions : dynamicOptions.length > 0 ? dynamicOptions : hintedOptions;
  if (options.length > 0) {
    return (
      <SelectField
        name={name}
        label={label}
        hintText={hintText}
        template={template}
        value={displayValue}
        options={options as SelectOption[]}
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
      />
    </div>
  );
}

/** Select with floating label + tooltips on every option (`title`). */
function SelectField({
  name,
  label,
  hintText,
  value,
  options,
  onChange,
}: {
  name: string;
  label: string;
  hintText: string;
  template?: boolean;
  value: string;
  options: SelectOption[];
  onChange: (value: JsonValue) => void;
}) {
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

/** Headers-style editor: keys are plain, values get template autocomplete. */
function KeyValueEditor({
  locale,
  label,
  hintText,
  entries,
  suggestions,
  onChange,
}: {
  locale: Locale;
  label: string;
  hintText: string;
  placeholder?: string;
  entries: JsonObject;
  suggestions: AutocompleteItem[];
  onChange: (value: JsonValue) => void;
}) {
  const removeLabel = t(locale, 'removeHeader');
  const keyLabel = t(locale, 'headerNameLabel');
  const valueLabel = t(locale, 'headerValueLabel');
  return (
    <div class="plg-field">
      <div class="plg-label-row">
        <label class="plg-label">{label}</label>
        <InfoTip text={hintText || t(locale, 'headersDefaultHint')} position="right" />
      </div>
      {Object.entries(entries).map(([key, entry], index) => (
        <div class="plg-kv-row" key={`header-${index}`}>
          <input
            class="plg-input plg-input--mono plg-input--key"
            value={key}
            aria-label={keyLabel}
            data-tooltip={keyLabel}
            data-tooltip-pos="right"
            placeholder="content-type"
            onInput={(event) => {
              const nextName = (event.currentTarget as HTMLInputElement).value;
              const list = Object.entries(entries);
              const next: JsonObject = {};
              list.forEach(([currentKey, currentValue], currentIndex) => {
                next[currentIndex === index ? nextName : currentKey] = currentValue;
              });
              onChange(next);
            }}
          />
          <span class="plg-kv-row__value">
            <TemplateField
              locale={locale}
              value={String(entry ?? '')}
              onValueChange={(next) => {
                const list = Object.entries(entries);
                const nextEntries: JsonObject = {};
                list.forEach(([currentKey, currentValue], currentIndex) => {
                  nextEntries[currentKey] = currentIndex === index ? next : currentValue;
                });
                onChange(nextEntries);
              }}
              suggestions={suggestions}
              ariaLabel={valueLabel}
              label={valueLabel}
            />
          </span>
          <button
            type="button"
            class="plg-btn plg-btn--icon plg-btn--danger"
            aria-label={removeLabel}
            data-tooltip={removeLabel}
            data-tooltip-pos="left"
            onClick={() => {
              const next = { ...entries };
              delete next[key];
              onChange(next);
            }}
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        class="plg-btn plg-btn--sm"
        data-tooltip={t(locale, 'addHeaderTooltip')}
        data-tooltip-pos="bottom"
        onClick={() => onChange({ ...entries, [`field-${Object.keys(entries).length + 1}`]: '' })}
      >
        + {t(locale, 'add')}
      </button>
    </div>
  );
}

function objectProperties(value: JsonValue | undefined): Record<string, JsonObject> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(Object.entries(value).filter((entry): entry is [string, JsonObject] => Boolean(entry[1]) && typeof entry[1] === 'object' && !Array.isArray(entry[1])));
}

function localized(value: JsonValue | undefined, locale: Locale): string {
  return i18nText(locale, value);
}

function applies(value: JsonValue | undefined, config: JsonObject): boolean {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return true;
  const condition = value as JsonObject;
  const key = typeof condition.key === 'string' ? condition.key : '';
  if (!key) return true;
  const current = String(config[key] ?? '');
  const equals = Array.isArray(condition.equals) ? condition.equals : [];
  const notEquals = Array.isArray(condition.notEquals) ? condition.notEquals : [];
  if (equals.length > 0 && !equals.some((entry) => String(entry) === current)) return false;
  if (notEquals.some((entry) => String(entry) === current)) return false;
  return true;
}

function toDisplayValue(value: JsonValue | undefined, type: JsonValue | undefined): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return type === 'object' || type === 'array' ? formatJson(value) : JSON.stringify(value);
}

function formatJson(value: JsonValue | undefined): string {
  if (value === undefined) return '';
  try { return JSON.stringify(value, null, 2) ?? ''; } catch { return ''; }
}

function schemaFromFields(type: ActionTypeDefinition): JsonObject {
  const properties: JsonObject = {};
  for (const field of type.fields ?? []) properties[field.key] = { type: field.kind === 'number' || field.kind === 'range' ? 'number' : field.kind === 'boolean' ? 'boolean' : field.kind === 'keyvalue' ? 'object' : 'string', title: field.label, default: field.value, minimum: field.min, maximum: field.max, multipleOf: field.step };
  return { type: 'object', properties };
}

function hintsFromFields(type: ActionTypeDefinition): JsonObject {
  const fields: JsonObject = {};
  for (const field of type.fields ?? []) fields[field.key] = { kind: field.kind, placeholder: field.placeholder, template: field.template, advanced: field.advanced, hint: field.hint, showIf: field.showIf, options: field.options } as unknown as JsonValue;
  return { fields };
}

export default SchemaForm;
</script>
