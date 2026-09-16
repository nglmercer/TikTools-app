<script lang="tsx">
import { TemplateField } from '../node-editor/TemplateField.vue';
import { getFetchUrlTemplates, isLocalFetchUrl } from '../node-editor/template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { InfoTip } from '../ui/InfoTip.vue';
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
      <span class="act-label">
        {urlLabel}
        {urlHint ? <InfoTip text={urlHint} position="right" /> : null}
      </span>
      <div class="act-endpoint">
        <select
          class="act-method"
          name="method"
          value={currentMethod}
          aria-label={methodLabel}
          onChange={(event) => onMethodChange((event.currentTarget as HTMLSelectElement).value)}
        >
          {methodOptions.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
        <TemplateField
          locale={locale}
          name="url"
          value={url}
          onValueChange={onUrlChange}
          suggestions={urlSuggestions}
          ariaLabel={urlLabel}
          placeholder={urlPlaceholder ?? 'https://'}
          bareWordTrigger={false}
          urlPresets={getFetchUrlTemplates()}
        />
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
