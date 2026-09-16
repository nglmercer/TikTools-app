<script lang="tsx">
import { TemplateField } from '../node-editor/TemplateField.vue';
import { InfoTip } from './InfoTip.vue';
import { Tooltip } from './Tooltip.vue';
import { IconClose } from '../icons/index.ts';
import { t, type Locale } from '../../i18n.ts';
import type { JsonObject, JsonValue } from '../../../automation/types.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';

/** Headers-style editor: keys are plain, values get template autocomplete. */
export function KeyValueEditor({
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
          <Tooltip text={keyLabel} position="right">
            <input
              class="plg-input plg-input--mono plg-input--key"
              value={key}
              aria-label={keyLabel}
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
          </Tooltip>
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
          <Tooltip text={removeLabel} position="left">
            <button
              type="button"
              class="plg-btn plg-btn--icon plg-btn--danger"
              aria-label={removeLabel}
              onClick={() => {
                const next = { ...entries };
                delete next[key];
                onChange(next);
              }}
            >
              <IconClose size={10} />
            </button>
          </Tooltip>
        </div>
      ))}
      <Tooltip text={t(locale, 'addHeaderTooltip')} position="bottom">
        <button
          type="button"
          class="plg-btn plg-btn--sm"
          onClick={() => onChange({ ...entries, [`field-${Object.keys(entries).length + 1}`]: '' })}
        >
          + {t(locale, 'add')}
        </button>
      </Tooltip>
    </div>
  );
}

export default KeyValueEditor;
</script>
