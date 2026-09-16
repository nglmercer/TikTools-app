<script lang="tsx">
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { KeyValueEditor } from '../ui/SchemaForm.vue';
import type { JsonObject } from '../../../automation/types.ts';
import type { Locale } from '../../i18n.ts';

type HttpHeadersEditorProps = {
  locale: Locale;
  label: string;
  hint?: string;
  headers: JsonObject;
  suggestions: AutocompleteItem[];
  onChange: (headers: JsonObject) => void;
};

/** Header rows shared by Behavior fetch and workflow HTTP nodes. */
export function HttpHeadersEditor({ locale, label, hint, headers, suggestions, onChange }: HttpHeadersEditorProps) {
  return (
    <div class="act-headers">
      <KeyValueEditor
        locale={locale}
        label={label}
        hintText={hint ?? ''}
        entries={headers}
        suggestions={suggestions}
        onChange={(value) => {
          if (value && typeof value === 'object' && !Array.isArray(value)) onChange(value as JsonObject);
        }}
      />
    </div>
  );
}

export default HttpHeadersEditor;
</script>
