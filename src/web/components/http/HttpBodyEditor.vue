<script lang="tsx">
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { CodeEditor, formatJsonText } from '../ui/CodeEditor.vue';
import { IconFormat } from '../icons/index.ts';
import { t, type Locale } from '../../i18n.ts';
import { HTTP_BODY_MODES, type HttpBodyMode } from './http-request.ts';

type HttpBodyEditorProps = {
  locale: Locale;
  mode: HttpBodyMode;
  body: string;
  onBodyChange: (body: string) => void;
  onModeChange: (mode: HttpBodyMode) => void;
  suggestions: AutocompleteItem[];
  ariaLabel: string;
};

/**
 * Request body with an explicit JSON/Text mode. Switching modes only changes
 * highlighting and validation — the body text is never rewritten.
 */
export function HttpBodyEditor({
  locale,
  mode,
  body,
  onBodyChange,
  onModeChange,
  suggestions,
  ariaLabel,
}: HttpBodyEditorProps) {
  const isJson = mode === 'json';
  const formatBody = (): void => {
    const formatted = formatJsonText(body);
    if (formatted !== null && formatted !== body) onBodyChange(formatted);
  };
  return (
    <div class="http-body">
      <div class="http-body__toolbar">
        <div class="http-body__modes" role="group" aria-label={t(locale, 'httpBodyMode')}>
          {HTTP_BODY_MODES.map((candidate) => (
            <button
              key={candidate}
              type="button"
              class={`http-body__mode${candidate === mode ? ' is-active' : ''}`}
              aria-pressed={candidate === mode}
              onClick={() => onModeChange(candidate)}
            >
              {candidate === 'json' ? t(locale, 'httpBodyJson') : t(locale, 'httpBodyText')}
            </button>
          ))}
        </div>
        {isJson && (
          <button type="button" class="act-format" onClick={formatBody}>
            <span aria-hidden="true" class="http-body__format-icon"><IconFormat size={14} /></span> {t(locale, 'behavior.editor.format')}
          </button>
        )}
      </div>
      <CodeEditor
        locale={locale}
        name="body"
        language={isJson ? 'json' : 'text'}
        value={body}
        onValueChange={onBodyChange}
        suggestions={suggestions}
        filename={isJson ? 'payload.json' : 'payload.txt'}
        mime={isJson ? 'application/json' : 'text/plain'}
        rows={7}
        ariaLabel={ariaLabel}
        validateJson={isJson}
      />
    </div>
  );
}

export default HttpBodyEditor;
</script>
