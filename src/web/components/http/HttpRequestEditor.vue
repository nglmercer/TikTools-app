<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import type { JsonObject } from '../../../automation/types.ts';
import { Checkbox } from '../ui/Checkbox.vue';
import { NumberInput } from '../ui/NumberInput.vue';
import { Select } from '../ui/Select.vue';
import { TemplateField } from '../node-editor/TemplateField.vue';
import { t, type Locale } from '../../i18n.ts';
import { HttpEndpointField, type HttpMethodOption } from './HttpEndpointField.vue';
import { HttpHeadersEditor } from './HttpHeadersEditor.vue';
import { HttpBodyEditor } from './HttpBodyEditor.vue';
import {
  buildHttpHeaders,
  getHeader,
  normalizeHttpMethod,
  readHttpBodyMode,
  suggestContentType,
  type HttpBodyMode,
} from './http-request.ts';

export type { HttpMethodOption };

type HttpRequestEditorProps = {
  locale: Locale;
  config: JsonObject;
  onPatchConfig: (patch: JsonObject) => void;
  methodOptions: HttpMethodOption[];
  defaultMethod?: string;
  methodLabel: string;
  urlLabel: string;
  urlHint?: string;
  urlPlaceholder?: string;
  urlSuggestions: AutocompleteItem[];
  bodyLabel: string;
  bodySuggestions: AutocompleteItem[];
  defaultBodyMode?: HttpBodyMode;
  headersLabel: string;
  headersHint?: string;
  headerSuggestions: AutocompleteItem[];
  timeoutLabel: string;
  timeoutHint?: string;
  allowPrivateLabel: string;
  allowPrivateHint?: string;
  showResponseType?: boolean;
  responseTypeLabel?: string;
  showRedirect?: boolean;
  redirectLabel?: string;
  redirectFollowLabel?: string;
  redirectBlockLabel?: string;
  showEmitResponseAs?: boolean;
  emitResponseAsLabel?: string;
  emitResponseAsHint?: string;
  emitResponseAsPlaceholder?: string;
  emitResponseAsSuggestions?: AutocompleteItem[];
  extraAdvanced?: VNodeChild;
};

/**
 * Shared HTTP request editor for Behavior `core.fetch` and workflow
 * `action.http`. Capability props enable the fields each host schema supports;
 * the component never branches on which consumer it serves.
 */
export const HttpRequestEditor = defineVueComponent<HttpRequestEditorProps>(
  ['locale', 'config', 'onPatchConfig', 'methodOptions', 'defaultMethod', 'methodLabel', 'urlLabel', 'urlHint', 'urlPlaceholder', 'urlSuggestions', 'bodyLabel', 'bodySuggestions', 'defaultBodyMode', 'headersLabel', 'headersHint', 'headerSuggestions', 'timeoutLabel', 'timeoutHint', 'allowPrivateLabel', 'allowPrivateHint', 'showResponseType', 'responseTypeLabel', 'showRedirect', 'redirectLabel', 'redirectFollowLabel', 'redirectBlockLabel', 'showEmitResponseAs', 'emitResponseAsLabel', 'emitResponseAsHint', 'emitResponseAsPlaceholder', 'emitResponseAsSuggestions', 'extraAdvanced'],
  (props) => {
  const tab = ref<'body' | 'headers' | 'auth'>('body');

  return () => {
    const { locale, config, onPatchConfig } = props;
    const method = normalizeHttpMethod(config.method, props.defaultMethod ?? 'POST');
    const isGet = method === 'GET';
    const activeTab: 'body' | 'headers' | 'auth' = isGet && tab.value === 'body' ? 'headers' : tab.value;
    const headers = config.headers && typeof config.headers === 'object' && !Array.isArray(config.headers)
      ? (config.headers as JsonObject)
      : {};
    const headerCount = Object.keys(headers).length;
    const body = typeof config.body === 'string' ? config.body : '';
    const url = typeof config.url === 'string' ? config.url : '';
    const allowPrivate = config.allowPrivateNetwork === true || config.allowPrivateNetwork === 'true';
    const bodyMode = readHttpBodyMode(config, props.defaultBodyMode ?? 'json');
    const timeout = typeof config.timeoutMs === 'number' && Number.isFinite(config.timeoutMs) ? config.timeoutMs : null;
    const advancedSummary = [
      props.timeoutLabel,
      props.showResponseType ? (props.responseTypeLabel ?? '') : '',
      props.showRedirect ? (props.redirectLabel ?? '') : '',
      props.showEmitResponseAs ? (props.emitResponseAsLabel ?? '') : '',
      props.allowPrivateLabel,
    ].filter(Boolean).slice(0, 3).join(', ');

    const switchBodyMode = (mode: HttpBodyMode): void => {
      if (mode === bodyMode) return;
      const patch: JsonObject = { bodyMode: mode };
      if (getHeader(headers, 'content-type') === undefined) {
        patch.headers = buildHttpHeaders(headers, { 'Content-Type': suggestContentType(mode) });
      }
      onPatchConfig(patch);
    };

    return (
      <div class="act-fetch">
        <HttpEndpointField
          locale={locale}
          method={typeof config.method === 'string' ? config.method : undefined}
          url={url}
          defaultMethod={props.defaultMethod}
          methodOptions={props.methodOptions}
          methodLabel={props.methodLabel}
          urlLabel={props.urlLabel}
          urlHint={props.urlHint}
          urlPlaceholder={props.urlPlaceholder}
          allowPrivateNetwork={allowPrivate}
          onMethodChange={(next) => onPatchConfig({ method: next })}
          onUrlChange={(next) => onPatchConfig({ url: next })}
          onEnablePrivateNetwork={() => onPatchConfig({ allowPrivateNetwork: true })}
          urlSuggestions={props.urlSuggestions}
        />

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
                {bodyMode === 'json' ? t(locale, 'behavior.editor.bodyTab') : t(locale, 'httpBodyTabText')}
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
        </div>

        {activeTab === 'body' && (
          <HttpBodyEditor
            locale={locale}
            mode={bodyMode}
            body={body}
            onBodyChange={(next) => onPatchConfig({ body: next })}
            onModeChange={switchBodyMode}
            suggestions={props.bodySuggestions}
            ariaLabel={props.bodyLabel}
          />
        )}

        {activeTab === 'headers' && (
          <HttpHeadersEditor
            locale={locale}
            label={props.headersLabel}
            hint={props.headersHint}
            headers={headers}
            suggestions={props.headerSuggestions}
            onChange={(next) => onPatchConfig({ headers: next })}
          />
        )}

        {activeTab === 'auth' && (
          <div class="act-auth-empty">{t(locale, 'behavior.editor.authEmpty')}</div>
        )}

        <details class="plg-details act-adv">
          <summary>
            <span>{t(locale, 'behavior.copy.advanced')}</span>
            {advancedSummary && <span class="act-adv__summary">{advancedSummary}</span>}
          </summary>
          <div class="plg-details__body http-advanced">
            <NumberInput
              name="timeoutMs"
              label={props.timeoutLabel}
              hint={props.timeoutHint}
              value={timeout}
              onValueChange={(next) => onPatchConfig({ timeoutMs: next })}
              suffix="ms"
            />
            {props.showResponseType && (
              <Select
                name="responseType"
                label={props.responseTypeLabel ?? 'Response'}
                value={typeof config.responseType === 'string' ? config.responseType : 'auto'}
                options={[{ value: 'auto', label: 'Auto' }, { value: 'json', label: 'JSON' }, { value: 'text', label: 'Text' }, { value: 'bytes', label: 'Bytes' }]}
                onValueChange={(next) => onPatchConfig({ responseType: next })}
              />
            )}
            {props.showRedirect && (
              <Select
                name="redirect"
                label={props.redirectLabel ?? 'Redirects'}
                value={typeof config.redirect === 'string' ? config.redirect : 'error'}
                options={[
                  { value: 'error', label: props.redirectBlockLabel ?? 'Block redirects' },
                  { value: 'follow', label: props.redirectFollowLabel ?? 'Follow redirects' },
                ]}
                onValueChange={(next) => onPatchConfig({ redirect: next })}
              />
            )}
            {props.showEmitResponseAs && (
              <TemplateField
                locale={locale}
                name="emitResponseAs"
                label={props.emitResponseAsLabel ?? 'Emit the response as'}
                hint={props.emitResponseAsHint}
                value={typeof config.emitResponseAs === 'string' ? config.emitResponseAs : ''}
                onValueChange={(next) => onPatchConfig({ emitResponseAs: next })}
                suggestions={props.emitResponseAsSuggestions ?? []}
                ariaLabel={props.emitResponseAsLabel ?? 'Emit the response as'}
                placeholder={props.emitResponseAsPlaceholder}
              />
            )}
            <Checkbox
              name="allowPrivateNetwork"
              checked={allowPrivate}
              onCheckedChange={(next) => onPatchConfig({ allowPrivateNetwork: next })}
              label={props.allowPrivateLabel}
              ariaLabel={props.allowPrivateHint ? undefined : props.allowPrivateLabel}
            />
            {props.extraAdvanced}
          </div>
        </details>
      </div>
    );
  };
  },
);

export default HttpRequestEditor;
</script>
