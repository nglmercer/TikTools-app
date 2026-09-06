<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { PermissionCards, TestConsole } from '../../components/ui/FieldPanels.vue';
import { SchemaForm, schemaForAction, resolveAutocompleteSources } from '../../components/ui/SchemaForm.vue';
import { CodeEditor, formatJsonText } from '../../components/ui/CodeEditor.vue';
import { TemplateField } from '../../components/node-editor/TemplateField.vue';
import { TextInput } from '../../components/ui/TextInput.vue';
import { InfoTip } from '../../components/ui/InfoTip.vue';
import {
  getFetchUrlTemplates,
  isLocalFetchUrl,
  type TemplateSuggestionScope,
} from '../../components/node-editor/template-suggestions.ts';
import {
  deriveActionPermissions,
  readString,
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
  onGetActionOptions: (source: string) => void;
  onOpenMediaPicker: OpenMediaPicker;
  onCancel: () => void;
  onSave: (action: LiveAction) => void;
  onDelete: (id: string) => void;
  onTest: (action: LiveAction, trigger?: string) => void;
};

export const ActionEditor = defineVueComponent<ActionEditorProps>(
  ['locale', 'action', 'actionTypes', 'isNew', 'error', 'testRuns', 'actionOptions', 'onGetActionOptions', 'onOpenMediaPicker', 'onCancel', 'onSave', 'onDelete', 'onTest'],
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
        <button
          type="button"
          class="plg-btn plg-btn--icon"
          onClick={props.onCancel}
          aria-label={t(props.locale, 'behavior.copy.back')}
          data-tooltip={t(props.locale, 'behavior.copy.backHint')}
          data-tooltip-pos="bottom"
          data-tooltip-wide=""
        >
          ‹
        </button>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{draftValue.name || t(props.locale, 'behavior.copy.newAction')}</h2>
          <span
            class="plg-topbar__subtitle plg-mono"
            data-tooltip={typeValue ? `${i18nText(props.locale, typeValue.description)}${t(props.locale, 'behavior.copy.typeHint') ? ` — ${t(props.locale, 'behavior.copy.typeHint')}` : ''}` : draftValue.typeId}
            data-tooltip-pos="bottom"
            data-tooltip-wide=""
          >
            {typeValue ? `${originLabel(typeValue, props.locale, t(props.locale, 'behavior.copy.builtIn'))} · ${typeValue.tag}` : draftValue.typeId}
          </span>
        </div>
        <div class="plg-topbar__actions">
          {!props.isNew && (
            <button
              type="button"
              class="plg-btn plg-btn--danger plg-btn--sm"
              data-tooltip={t(props.locale, 'behavior.copy.deleteHint')}
              data-tooltip-pos="bottom"
              data-tooltip-wide=""
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
          )}
          <button
            type="button"
            class="plg-btn plg-btn--primary plg-btn--sm"
            data-tooltip={t(props.locale, 'behavior.copy.saveHint')}
            data-tooltip-pos="bottom"
            data-tooltip-wide=""
            onClick={() => props.onSave(draftValue)}
          >
            {t(props.locale, 'behavior.copy.save')}
          </button>
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
                      onClick={() => { for (const field of dynamicFieldsValue) props.onGetActionOptions(field.source); }}
                    >
                      {t(locale, 'behavior.copy.refreshOptions')}
                    </button>
                  </div>
                )}
                {isFetch ? (
                  <FetchFields
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
/** Endpoint + tabbed body layout for `core.fetch`, following the webhook-editor mockup. */
type FetchFieldsProps = {
  locale: Locale;
  draft: LiveAction;
  form: { schema: JsonObject; uiHints?: JsonObject };
  suggestionsFor: (name: string, template: boolean) => Array<{ value: string; label: string }>;
  suggestionContext: JsonObject;
  onOpenMediaPicker?: OpenMediaPicker;
  onPatchConfig: (patch: JsonObject) => void;
};

const FetchFields = defineVueComponent<FetchFieldsProps>(
  ['locale', 'draft', 'form', 'suggestionsFor', 'suggestionContext', 'onOpenMediaPicker', 'onPatchConfig'],
  (props) => {
  const tab = ref<'body' | 'headers' | 'auth'>('body');

  return () => {
  const { locale, draft, form, suggestionsFor, suggestionContext, onPatchConfig } = props;
  const method = readString(draft.config.method) || 'POST';
  const isGet = method.toUpperCase() === 'GET';
  const activeTab: 'body' | 'headers' | 'auth' = isGet && tab.value === 'body' ? 'headers' : tab.value;
  const headers = readStringMap(draft.config.headers);
  const headerCount = Object.keys(headers).length;
  const body = readString(draft.config.body);

  const properties = objectPropertiesOf(form.schema.properties);
  const fieldHints = objectPropertiesOf(
    form.uiHints && typeof form.uiHints.fields === 'object' && !Array.isArray(form.uiHints.fields)
      ? form.uiHints.fields as JsonObject
      : undefined,
  );
  const advancedKeys = Object.keys(properties).filter((key) => {
    const hint = fieldHints[key];
    return hint !== undefined && (hint as JsonObject).advanced === true && key !== 'headers';
  });
  const advancedSummary = advancedKeys
    .map((key) => fieldTitle(properties[key], locale) || key)
    .slice(0, 3)
    .join(', ');
  const headersForm = stripAdvanced(pickForm(form, ['headers']));
  const advancedForm = stripAdvanced(pickForm(form, advancedKeys));

  const formatBody = (): void => {
    const formatted = formatJsonText(body);
    if (formatted !== null && formatted !== body) onPatchConfig({ body: formatted });
  };

  const urlValue = readString(draft.config.url);
  const allowPrivate = readString(draft.config.allowPrivateNetwork) === 'true';
  const showLocalHint = urlValue.trim().length > 0 && isLocalFetchUrl(urlValue) && !allowPrivate;
  const urlPresets = getFetchUrlTemplates();

  return (
    <div class="act-fetch">
      <div class="plg-field">
        <span class="act-label">
          {t(locale, 'behavior.editor.endpoint')}
          {fieldHint(fieldHints.url, locale) && <InfoTip text={fieldHint(fieldHints.url, locale)} position="right" />}
        </span>
        <div class="act-endpoint">
          <select
            class="act-method"
            name="method"
            value={method}
            aria-label={fieldTitle(properties.method, locale) || 'Method'}
            onChange={(event) => onPatchConfig({ method: (event.currentTarget as HTMLSelectElement).value })}
          >
            {methodOptions(properties.method, fieldHints.method, locale).map((option) => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </select>
          <TemplateField
            locale={locale}
            name="url"
            value={urlValue}
            onValueChange={(next) => onPatchConfig({ url: next })}
            suggestions={suggestionsFor('url', true)}
            ariaLabel={fieldTitle(properties.url, locale) || 'URL'}
            placeholder={fieldPlaceholder(fieldHints.url) ?? 'https://'}
            bareWordTrigger={false}
            urlPresets={urlPresets}
          />
        </div>
        {showLocalHint && (
          <p class="act-localhint" role="note">
            <span>{t(locale, 'behavior.editor.localNetHint')}</span>
            <button type="button" class="act-preset" onClick={() => onPatchConfig({ allowPrivateNetwork: true })}>
              {t(locale, 'behavior.editor.enableLocalNet')}
            </button>
          </p>
        )}
      </div>

      <div class="act-tabrow">
        <div class="act-tabs" role="tablist">
          {!isGet && (
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'body'}
              class={`act-tab${activeTab === 'body' ? ' is-active' : ''}`}
              onClick={() => { tab.value = 'body'; }}
            >
              {t(locale, 'behavior.editor.bodyTab')}
            </button>
          )}
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'headers'}
            class={`act-tab${activeTab === 'headers' ? ' is-active' : ''}`}
            onClick={() => { tab.value = 'headers'; }}
          >
            {t(locale, 'behavior.editor.headersTab')}
            {headerCount > 0 && <span class="act-tabcount">{headerCount}</span>}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'auth'}
            class={`act-tab${activeTab === 'auth' ? ' is-active' : ''}`}
            onClick={() => { tab.value = 'auth'; }}
          >
            {t(locale, 'behavior.editor.authTab')}
          </button>
        </div>
        {activeTab === 'body' && (
          <button type="button" class="act-format" onClick={formatBody}>
            <span aria-hidden="true">☰</span> {t(locale, 'behavior.editor.format')}
          </button>
        )}
      </div>

      {activeTab === 'body' && (
        <CodeEditor
          locale={locale}
          name="body"
          language="json"
          value={body}
          onValueChange={(next) => onPatchConfig({ body: next })}
          suggestions={suggestionsFor('body', true)}
          filename="payload.json"
          mime="application/json"
          rows={7}
          ariaLabel={fieldTitle(properties.body, locale) || 'Body'}
        />
      )}

      {activeTab === 'headers' && (
        <div class="act-headers">
          <SchemaForm
            locale={locale}
            schema={headersForm.schema}
            uiHints={headersForm.uiHints}
            value={draft.config}
            suggestionContext={suggestionContext}
            suggestionScopes={{ headers: 'http-data' }}
            onOpenMediaPicker={props.onOpenMediaPicker}
            onChange={(config) => onPatchConfig({ headers: config.headers ?? {} })}
          />
        </div>
      )}

      {activeTab === 'auth' && (
        <div class="act-auth-empty">{t(locale, 'behavior.editor.authEmpty')}</div>
      )}

      {advancedKeys.length > 0 && (
        <details class="plg-details act-adv">
          <summary>
            <span>{t(locale, 'behavior.copy.advanced')}</span>
            {advancedSummary && <span class="act-adv__summary">{advancedSummary}</span>}
          </summary>
          <div class="plg-details__body">
            <SchemaForm
              locale={locale}
              schema={advancedForm.schema}
              uiHints={advancedForm.uiHints}
              value={draft.config}
              onOpenMediaPicker={props.onOpenMediaPicker}
              onChange={(config) => onPatchConfig(config)}
            />
          </div>
        </details>
      )}
    </div>
  );
  };
  },
);

export default ActionEditor;
</script>
