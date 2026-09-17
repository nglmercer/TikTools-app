<script lang="tsx">
import { SelectField } from '../ui/fields/SelectField.vue';
import { TemplateField } from '../ui/fields/TemplateField.vue';
import { getFetchUrlTemplates, isLocalFetchUrl } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { t, type Locale } from '../../i18n.ts';
import { normalizeHttpMethod } from './http-request.ts';

export type HttpMethodOption = {
  value: string;
  label: string;
};

type HttpEndpointFieldProps = {
  locale: Locale;
  method: string | undefined;
  url: string;
  defaultMethod?: string;
  methodOptions: HttpMethodOption[];
  methodLabel: string;
  urlLabel: string;
  urlHint?: string;
  urlPlaceholder?: string;
  allowPrivateNetwork: boolean;
  onMethodChange: (method: string) => void;
  onUrlChange: (url: string) => void;
  onEnablePrivateNetwork: () => void;
  urlSuggestions: AutocompleteItem[];
};

/** Method select + URL template field with the local-network warning. */
export function HttpEndpointField({
  locale,
  method,
  url,
  defaultMethod = 'POST',
  methodOptions,
  methodLabel,
  urlLabel,
  urlHint,
  urlPlaceholder,
  allowPrivateNetwork,
  onMethodChange,
  onUrlChange,
  onEnablePrivateNetwork,
  urlSuggestions,
}: HttpEndpointFieldProps) {
  const currentMethod = normalizeHttpMethod(method, defaultMethod);
  const showLocalHint = url.trim().length > 0 && isLocalFetchUrl(url) && !allowPrivateNetwork;
  return (
    <div class="plg-field">
      <div class="act-endpoint">
        <div class="act-endpoint__method">
          <SelectField
            name="method"
            ariaLabel={methodLabel}
            value={currentMethod}
            options={methodOptions}
            onValueChange={onMethodChange}
            locale={locale}
          />
        </div>
        <div class="act-endpoint__url">
          <TemplateField
            locale={locale}
            name="url"
            label={urlLabel}
            hint={urlHint}
            value={url}
            onValueChange={onUrlChange}
            suggestions={urlSuggestions}
            scope="http-url"
            presets={getFetchUrlTemplates()}
            placeholder={urlPlaceholder ?? 'https://'}
          />
        </div>
      </div>
      {showLocalHint && (
        <p class="act-localhint" role="note">
          <span>{t(locale, 'behavior.editor.localNetHint')}</span>
          <button type="button" class="act-preset" onClick={onEnablePrivateNetwork}>
            {t(locale, 'behavior.editor.enableLocalNet')}
          </button>
        </p>
      )}
    </div>
  );
}

export default HttpEndpointField;
</script>
