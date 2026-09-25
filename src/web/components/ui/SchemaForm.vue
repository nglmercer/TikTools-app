<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { readFormValues } from './control-events.ts';
import type { AutomationEvent, AutomationEventType, JsonObject, JsonValue } from '../../../automation/types.ts';
import type { TemplateSuggestionScope } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { AdvancedSection } from './FieldPanels.vue';
import { t, type Locale } from '../../i18n.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { SchemaField } from './SchemaField.vue';
import {
  applies,
  acceptSchemaFieldValue,
  formSchemaFromJsonSchema,
  objectProperties,
  resolveAutocompleteSources,
  type FieldOption,
} from './schema-form-helpers.ts';

export type SchemaFormProps = {
  locale: Locale;
  schema: JsonObject;
  uiHints?: JsonObject;
  value: JsonObject;
  onChange: (value: JsonObject) => void;
  /** Additional suggestions merged with the host-provided context. */
  templateSuggestions?: AutocompleteItem[];
  /** Runtime globals merged as `globals.*` autocomplete rows. */
  globals?: Record<string, string>;
  /** Any object pushed as autocomplete (live event, custom schema sample…). */
  suggestionContext?: JsonValue | AutomationEvent;
  /** Per-field scopes, e.g. `{ url: 'http-url', body: 'http-data' }`. */
  suggestionScopes?: Partial<Record<string, TemplateSuggestionScope>>;
  /** Trigger used to pick trigger-specific paths when no explicit list given. */
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  /** Dynamic per-field options fetched on demand (voices, devices, …). */
  fieldOptions?: Record<string, FieldOption[]>;
  /** Inline validation messages keyed by field name. */
  fieldErrors?: Record<string, string>;
  /** Opens a host-owned native media dialog and returns a path reference. */
  onOpenMediaPicker?: OpenMediaPicker;
};

export type SchemaFormHandle = {
  getValues: () => Record<string, unknown>;
};
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
    'globals',
    'suggestionContext',
    'suggestionScopes',
    'eventType',
    'lastEvent',
    'fieldOptions',
    'fieldErrors',
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
  const fieldErrors = props.fieldErrors ?? {};
  const hints = objectProperties(props.uiHints?.fields);
  const visible = Object.entries(properties.value).filter(([key]) => applies(hints[key]?.showIf, props.value));
  const basic = visible.filter(([key]) => hints[key]?.advanced !== true);
  const advanced = visible.filter(([key]) => hints[key]?.advanced === true);
  // Schema-aware gate: a bubbled DOM Event must never enter settings state.
  // Reject by declared scalar type (never log `next`: fields can be secret).
  const update = (key: string, field: JsonObject, next: JsonValue): void => {
    if (!acceptSchemaFieldValue(field, next)) {
      console.warn(`[SchemaForm] rejected non-${String(field.type)} value for "${key}"`);
      return;
    }
    props.onChange({ ...props.value, [key]: next });
  };

  const suggestionsFor = resolveAutocompleteSources({
    locale: props.locale,
    suggestionContext: props.suggestionContext,
    suggestionScopes,
    eventType: props.eventType,
    lastEvent: props.lastEvent,
    templateSuggestions,
    globals: props.globals,
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
          onChange={(next) => update(key, field, next)}
          templateSuggestions={suggestionsFor(key, (hints[key]?.template as boolean) === true)}
          fieldOptions={fieldOptions[key]}
          error={fieldErrors[key]}
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
              onChange={(next) => update(key, field, next)}
              templateSuggestions={suggestionsFor(key, (hints[key]?.template as boolean) === true)}
              fieldOptions={fieldOptions[key]}
              error={fieldErrors[key]}
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
export { KeyValueEditor } from './KeyValueEditor.vue';
export { resolveAutocompleteSources, schemaForAction, type FieldOption } from './schema-form-helpers.ts';

export default SchemaForm;
</script>
