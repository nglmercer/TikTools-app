<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { PermissionCards, TestConsole } from '../../components/ui/FieldPanels.vue';
import { SchemaForm, schemaForAction, resolveAutocompleteSources } from '../../components/ui/SchemaForm.vue';
import { IconChevronLeft } from '../../components/icons/index.ts';
import { TextInput } from '../../components/ui/TextInput.vue';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import { HttpRequestEditor } from '../../components/http/index.ts';
import type { TemplateSuggestionScope } from '../../components/node-editor/template-suggestions.ts';
import {
  deriveActionPermissions,
  readStringMap,
} from '../../../automation/behavior/schema.ts';
import { sampleEventFor } from '../../../automation/behavior/samples.ts';
import { fieldsWithOptions, fieldHint, fieldPlaceholder, fieldTitle, methodOptions, objectPropertiesOf, pickForm, stripAdvanced, originLabel } from './helpers.vue';
import type {
  ActionTypeDefinition,
  BehaviorRun,
  LiveAction,
} from '../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker } from '../../../shared/messages.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { useDialogs } from '../../composables/useDialogs.ts';

type ActionEditorProps = {
  locale: Locale;
  action: LiveAction;
  actionTypes: ActionTypeDefinition[];
  isNew: boolean;
  error?: string;
  testRuns: BehaviorRun[];
  actionOptions: Record<string, ActionOptionItem[]>;
  actionOptionErrors: Record<string, string>;
  onGetActionOptions: (source: string, refresh?: boolean) => void;
  onOpenMediaPicker: OpenMediaPicker;
  onCancel: () => void;
  onSave: (action: LiveAction) => void;
  onDelete: (id: string) => void;
  onTest: (action: LiveAction, trigger?: string) => void;
};

export const ActionEditor = defineVueComponent<ActionEditorProps>(
  ['locale', 'action', 'actionTypes', 'isNew', 'error', 'testRuns', 'actionOptions', 'actionOptionErrors', 'onGetActionOptions', 'onOpenMediaPicker', 'onCancel', 'onSave', 'onDelete', 'onTest'],
  (props) => {
  const draft = ref<LiveAction>(props.action);
  const dialogs = useDialogs();
  watch(() => props.action, (action) => { draft.value = action; });
  const type = computed(() => props.actionTypes.find((entry) => entry.id === draft.value.typeId));
  const permissions = computed(() => deriveActionPermissions(draft.value, type.value));
  const testRun = computed(() => props.testRuns.find((run) => run.actionId === draft.value.id) ?? props.testRuns[0]);
  const form = computed(() => type.value ? schemaForAction(type.value) : undefined);
  const dynamicFields = computed(() => fieldsWithOptions(form.value?.uiHints));
  watch(() => type.value?.id, () => {
    for (const field of dynamicFields.value) props.onGetActionOptions(field.source);
  }, { immediate: true });
  const fieldOptions = computed(() => {
    const merged: Record<string, Array<{ value: string; label: string }>> = {};
    for (const field of dynamicFields.value) {
      const options = props.actionOptions[field.source];
      if (options && options.length > 0) merged[field.key] = options;
    }
    return merged;
  });
  const optionErrors = computed(() => {
    const errors: Array<{ key: string; message: string }> = [];
    for (const field of dynamicFields.value) {
      const message = props.actionOptionErrors[field.source];
      if (message) errors.push({ key: field.key, message });
    }
    return errors;
  });

  // Generic autocomplete context: the registry sample event, so `{{ }}`
  // works even before the first live event arrives. Any object can be pushed
  // here — SchemaForm merges it with the trigger scope.
  const suggestionContext = { event: sampleEventFor('tiktok.gift') } as unknown as JsonObject;
  const suggestionScopes = computed<Partial<Record<string, TemplateSuggestionScope>>>(() => {
    if (draft.value.typeId === 'core.fetch') {
      return { url: 'http-url', body: 'http-data', headers: 'http-data', emitResponseAs: 'identity', uniqueId: 'identity' };
    }
    if (draft.value.typeId === 'core.emit') return { type: 'identity', data: 'http-data' };
    if (draft.value.typeId === 'core.points') return { uniqueId: 'identity' };
    if (draft.value.typeId === 'core.log') return { message: 'message' };
    return {};
  });

  return () => {
  const draftValue = draft.value;
  const typeValue = type.value;
  const formValue = form.value;
  const dynamicFieldsValue = dynamicFields.value;
  const permissionsValue = permissions.value;
  const testRunValue = testRun.value;
  const locale = props.locale;
  const suggestionsFor = resolveAutocompleteSources({ locale: props.locale, suggestionContext, suggestionScopes: suggestionScopes.value });
  const isFetch = draftValue.typeId === 'core.fetch';
  return (
    <div class="plg">
      <div class="plg-topbar">
        <Tooltip text={t(props.locale, 'behavior.copy.backHint')} position="bottom" wide>
          <button
            type="button"
            class="plg-btn plg-btn--icon"
            onClick={props.onCancel}
            aria-label={t(props.locale, 'behavior.copy.back')}
          >
            <IconChevronLeft size={16} />
          </button>
        </Tooltip>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{draftValue.name || t(props.locale, 'behavior.copy.newAction')}</h2>
          <Tooltip
            text={typeValue ? `${i18nText(props.locale, typeValue.description)}${t(props.locale, 'behavior.copy.typeHint') ? ` — ${t(props.locale, 'behavior.copy.typeHint')}` : ''}` : draftValue.typeId}
            position="bottom"
            wide
          >
            <span class="plg-topbar__subtitle plg-mono">
              {typeValue ? `${originLabel(typeValue, props.locale, t(props.locale, 'behavior.copy.builtIn'))} · ${typeValue.tag}` : draftValue.typeId}
            </span>
          </Tooltip>
        </div>
        <div class="plg-topbar__actions">
          {!props.isNew && (
            <Tooltip text={t(props.locale, 'behavior.copy.deleteHint')} position="bottom" wide>
              <button
                type="button"
                class="plg-btn plg-btn--danger plg-btn--sm"
                onClick={async () => {
                  const confirmed = await dialogs.confirm(t(props.locale, 'behavior.copy.confirmDeleteAction'), {
                    title: t(props.locale, 'behavior.copy.remove'),
                    confirmLabel: t(props.locale, 'behavior.copy.remove'),
                    cancelLabel: t(props.locale, 'cancel'),
                    danger: true,
                  });
                  if (confirmed) props.onDelete(draftValue.id);
                }}
              >
                {t(props.locale, 'behavior.copy.remove')}
              </button>
            </Tooltip>
          )}
          <Tooltip text={t(props.locale, 'behavior.copy.saveHint')} position="bottom" wide>
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--sm"
              onClick={() => props.onSave(draftValue)}
            >
              {t(props.locale, 'behavior.copy.save')}
            </button>
          </Tooltip>
        </div>
      </div>

      <div class="plg-scroll">
        <div class="plg-form">
          <div class="plg-form__main">
            {props.error && <div class="plg-alert">{props.error}</div>}

            <div class="plg-field">
              <label class="act-label" for="actionName">{t(props.locale, 'behavior.editor.actionName')}</label>
              <TextInput
                id="actionName"
                name="actionName"
                value={draftValue.name}
                placeholder={typeValue ? i18nText(props.locale, typeValue.title) : undefined}
                onValueChange={(next) => { draft.value = { ...draft.value, name: next }; }}
              />
            </div>

            {formValue ? (
              <>
                {dynamicFieldsValue.length > 0 && (
                  <div class="plg-row">
                    <button
                      type="button"
                      class="plg-btn plg-btn--sm"
                      onClick={() => { for (const field of dynamicFieldsValue) props.onGetActionOptions(field.source, true); }}
                    >
                      {t(locale, 'behavior.copy.refreshOptions')}
                    </button>
                  </div>
                )}
                {optionErrors.value.map((entry) => (
                  <div key={entry.key} class="plg-alert" role="status">{entry.message}</div>
                ))}
                {isFetch ? (
                  <BehaviorFetchFields
                    locale={props.locale}
                    draft={draftValue}
                    form={formValue}
                    suggestionsFor={suggestionsFor}
                    suggestionContext={suggestionContext}
                    onOpenMediaPicker={props.onOpenMediaPicker}
                    onPatchConfig={(patch) => { draft.value = { ...draft.value, config: { ...draft.value.config, ...patch } }; }}
                  />
                ) : (
                  <SchemaForm
                    locale={props.locale}
                    schema={formValue.schema}
                    uiHints={formValue.uiHints}
                    value={draftValue.config}
                    fieldOptions={fieldOptions.value}
                    suggestionContext={suggestionContext}
                    suggestionScopes={suggestionScopes.value}
                    onOpenMediaPicker={props.onOpenMediaPicker}
                    onChange={(config) => { draft.value = { ...draft.value, config }; }}
                  />
                )}
              </>
            ) : <div class="plg-alert">{draftValue.typeId}</div>}
          </div>

          <div class="plg-side act-side">
            <PermissionCards
              locale={props.locale}
              network={permissionsValue.network}
              capabilities={permissionsValue.capabilities}
              noneLabel={t(props.locale, 'behavior.copy.none')}
            />
            <TestConsole
              locale={props.locale}
              run={testRunValue}
              headers={isFetch ? readStringMap(draftValue.config.headers) : undefined}
              onRun={() => props.onTest(draftValue)}
              emptyLabel={t(props.locale, 'behavior.copy.consoleEmpty')}
            />
          </div>
        </div>
      </div>
    </div>
  );
  };
  },
);
/** `core.fetch` adapter: schema-driven labels feed the shared HTTP editor. */
type BehaviorFetchFieldsProps = {
  locale: Locale;
  draft: LiveAction;
  form: { schema: JsonObject; uiHints?: JsonObject };
  suggestionsFor: (name: string, template: boolean) => Array<{ value: string; label: string }>;
  suggestionContext: JsonObject;
  onOpenMediaPicker?: OpenMediaPicker;
  onPatchConfig: (patch: JsonObject) => void;
};

/** Config keys rendered natively by the shared editor; the rest stay generic. */
const NATIVE_HTTP_KEYS = new Set(['method', 'url', 'headers', 'body', 'timeoutMs', 'emitResponseAs', 'allowPrivateNetwork']);

function BehaviorFetchFields({
  locale,
  draft,
  form,
  suggestionsFor,
  suggestionContext,
  onOpenMediaPicker,
  onPatchConfig,
}: BehaviorFetchFieldsProps) {
  const properties = objectPropertiesOf(form.schema.properties);
  const fieldHints = objectPropertiesOf(
    form.uiHints && typeof form.uiHints.fields === 'object' && !Array.isArray(form.uiHints.fields)
      ? form.uiHints.fields as JsonObject
      : undefined,
  );
  const leftoverKeys = Object.keys(properties).filter((key) => {
    const hint = fieldHints[key];
    return hint !== undefined && (hint as JsonObject).advanced === true && !NATIVE_HTTP_KEYS.has(key);
  });
  const leftoverForm = stripAdvanced(pickForm(form, leftoverKeys));

  return (
    <HttpRequestEditor
      locale={locale}
      config={draft.config}
      onPatchConfig={onPatchConfig}
      methodOptions={methodOptions(properties.method, fieldHints.method, locale)}
      defaultMethod="POST"
      methodLabel={fieldTitle(properties.method, locale) || 'Method'}
      urlLabel={t(locale, 'behavior.editor.endpoint')}
      urlHint={fieldHint(fieldHints.url, locale) || undefined}
      urlPlaceholder={fieldPlaceholder(fieldHints.url) ?? 'https://'}
      urlSuggestions={suggestionsFor('url', true)}
      bodyLabel={fieldTitle(properties.body, locale) || 'Body'}
      bodySuggestions={suggestionsFor('body', true)}
      defaultBodyMode="json"
      headersLabel={fieldTitle(properties.headers, locale) || 'Headers'}
      headersHint={fieldHint(fieldHints.headers, locale) || undefined}
      headerSuggestions={suggestionsFor('headers', true)}
      timeoutLabel={fieldTitle(properties.timeoutMs, locale) || 'Timeout'}
      timeoutHint={fieldHint(fieldHints.timeoutMs, locale) || undefined}
      allowPrivateLabel={fieldTitle(properties.allowPrivateNetwork, locale) || 'Allow local network'}
      allowPrivateHint={fieldHint(fieldHints.allowPrivateNetwork, locale) || undefined}
      showEmitResponseAs
      emitResponseAsLabel={fieldTitle(properties.emitResponseAs, locale) || 'Emit the response as'}
      emitResponseAsHint={fieldHint(fieldHints.emitResponseAs, locale) || undefined}
      emitResponseAsPlaceholder={fieldPlaceholder(fieldHints.emitResponseAs)}
      emitResponseAsSuggestions={suggestionsFor('emitResponseAs', false)}
      extraAdvanced={leftoverKeys.length > 0 ? (
        <SchemaForm
          locale={locale}
          schema={leftoverForm.schema}
          uiHints={leftoverForm.uiHints}
          value={draft.config}
          suggestionContext={suggestionContext}
          onOpenMediaPicker={onOpenMediaPicker}
          onChange={onPatchConfig}
        />
      ) : undefined}
    />
  );
}

export default ActionEditor;
</script>
