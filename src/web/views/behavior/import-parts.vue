<script lang="tsx">
import { onMounted, onUnmounted, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { Icon } from '../../components/icons/Icon.vue';
import { Button } from '../../components/ui/Button.vue';
import { i18nText, type Locale } from '../../i18n.ts';
import type { RuleTemplate } from './rule-templates.ts';

export type ImportDropzoneProps = {
  locale: Locale;
  /** Translated dropzone title, hint, and button labels. */
  title: string;
  hint: string;
  browseLabel: string;
  pasteLabel: string;
  clipboardSource: string;
  pastePressKeys: string;
  unreadableMessage: string;
  icon?: string;
  accept?: string;
  disabled?: boolean;
  onContent: (text: string, source: string) => void;
  onReadError: (message: string) => void;
};

/**
 * Shared file/drop/clipboard picker for rule imports. Mounted only on
 * import screens, so its document paste listener never hijacks typing
 * elsewhere. Raw JSON is staged from files or the clipboard — never typed.
 */
export const ImportDropzone = defineVueComponent<ImportDropzoneProps>(
  ['locale', 'title', 'hint', 'browseLabel', 'pasteLabel', 'clipboardSource', 'pastePressKeys', 'unreadableMessage', 'icon', 'accept', 'disabled', 'onContent', 'onReadError'],
  (props) => {
    const dragActive = ref(false);
    const fileInput = ref<HTMLInputElement | null>(null);
    const dropzone = ref<HTMLDivElement | null>(null);
    const pasteHint = ref('');

    const readFile = (file: File): void => {
      const reader = new FileReader();
      reader.onload = () => {
        props.onContent(typeof reader.result === 'string' ? reader.result : '', file.name);
      };
      reader.onerror = () => {
        props.onReadError(props.unreadableMessage);
      };
      reader.readAsText(file);
    };

    const openFilePicker = (): void => {
      if (!props.disabled) fileInput.value?.click();
    };

    // Paste events need no clipboard permission, unlike readText(), which the
    // desktop webview denies. The document listener covers Ctrl+V anywhere on
    // the import screen (it holds no text inputs, so nothing is hijacked).
    const handleDocumentPaste = (event: ClipboardEvent): void => {
      const text = event.clipboardData?.getData('text');
      if (text) {
        event.preventDefault();
        props.onContent(text, props.clipboardSource);
      }
    };
    onMounted(() => document.addEventListener('paste', handleDocumentPaste));
    onUnmounted(() => document.removeEventListener('paste', handleDocumentPaste));

    const pasteFromClipboard = async (): Promise<void> => {
      try {
        const read = navigator.clipboard?.readText;
        if (!read) throw new Error('clipboard unavailable');
        props.onContent(await read.call(navigator.clipboard), props.clipboardSource);
      } catch {
        // Fall back to guided manual paste: focus the dropzone and let the
        // document paste listener stage whatever the user pastes with Ctrl+V.
        pasteHint.value = props.pastePressKeys;
        dropzone.value?.focus();
      }
    };

    return () => (
      <div>
        <div
          ref={dropzone}
          role="group"
          aria-label={props.title}
          tabindex={0}
          class={`rule-import-dropzone${dragActive.value ? ' is-dragging' : ''}`}
          onClick={() => openFilePicker()}
          onKeydown={(event: KeyboardEvent) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              openFilePicker();
            }
          }}
          onDragover={(event: DragEvent) => {
            event.preventDefault();
            dragActive.value = true;
          }}
          onDragleave={() => { dragActive.value = false; }}
          onDrop={(event: DragEvent) => {
            event.preventDefault();
            dragActive.value = false;
            const file = event.dataTransfer?.files?.[0];
            if (file && !props.disabled) readFile(file);
          }}
        >
          <Icon name={props.icon ?? 'template'} size={24} />
          <p class="rule-import-dropzone__title">{props.title}</p>
          <p class="rule-import-dropzone__hint">{props.hint}</p>
          <div class="rule-import-dropzone__actions">
            <span onClick={(event) => event.stopPropagation()}>
              <Button variant="soft" size="md" onClick={() => openFilePicker()}>
                {props.browseLabel}
              </Button>
            </span>
            <span onClick={(event) => event.stopPropagation()}>
              <Button variant="soft" size="md" onClick={() => void pasteFromClipboard()}>
                {props.pasteLabel}
              </Button>
            </span>
          </div>
          <input
            ref={fileInput}
            type="file"
            accept={props.accept ?? '.json,application/json'}
            hidden
            onChange={(event) => {
              const input = event.target as HTMLInputElement;
              const file = input.files?.[0];
              input.value = '';
              if (file) readFile(file);
            }}
          />
        </div>
        {pasteHint.value && (
          <p class="rule-import-hint" role="status">{pasteHint.value}</p>
        )}
      </div>
    );
  },
);

export type StagedTemplateListProps = {
  locale: Locale;
  /** Pre-translated headline, e.g. the profile name or template count. */
  headline: string;
  templates: RuleTemplate[];
  source: string;
  stagedFrom: string;
  clearLabel: string;
  /** Optional pre-translated note rendered under the list. */
  hint?: string;
  onClear: () => void;
};

/** Shared staged-import preview: what would be imported, before it is. */
export const StagedTemplateList = defineVueComponent<StagedTemplateListProps>(
  ['locale', 'headline', 'templates', 'source', 'stagedFrom', 'clearLabel', 'hint', 'onClear'],
  (props) => () => (
    <div class="rule-import-staged" role="status">
      <div class="rule-import-staged__head">
        <strong>{props.headline}</strong>
        <span class="rule-import-staged__source">{props.stagedFrom}</span>
        <Button variant="ghost" size="sm" onClick={() => props.onClear()}>
          {props.clearLabel}
        </Button>
      </div>
      <ul>
        {props.templates.map((template) => (
          <li key={template.id}>
            <Icon name={template.icon} size={14} />
            <span>{i18nText(props.locale, template.title)}</span>
            <small>{template.event.trigger}</small>
          </li>
        ))}
      </ul>
      {props.hint ? <p class="rule-import-hint">{props.hint}</p> : null}
    </div>
  ),
);

export default ImportDropzone;
</script>

<style scoped>
.rule-import-dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 28px 16px;
  text-align: center;
  border: 2px dashed var(--tt-border, #34343f);
  border-radius: 12px;
  background: var(--tt-bg-soft, #1c1c26);
  cursor: pointer;
  outline: none;
}

.rule-import-dropzone:focus-visible {
  border-color: var(--tt-accent, #22c55e);
}

.rule-import-dropzone.is-dragging {
  border-color: var(--tt-accent, #22c55e);
  border-style: solid;
}

.rule-import-dropzone__title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
}

.rule-import-dropzone__hint {
  margin: 0;
  font-size: 12px;
  color: var(--tt-text-dim, #a8a8b8);
}

.rule-import-dropzone__actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.rule-import-staged {
  border: 1px solid var(--tt-border, #34343f);
  border-radius: 12px;
  padding: 12px;
  background: var(--tt-bg-soft, #1c1c26);
}

.rule-import-staged__head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.rule-import-staged__source {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 12px;
}

.rule-import-staged ul {
  list-style: none;
  margin: 8px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-import-staged li {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.rule-import-staged li small {
  margin-left: auto;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 11px;
}

.rule-import-hint {
  margin: 8px 0 0;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 13px;
}
</style>
