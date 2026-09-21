<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';

import type { AutomationScriptCompletion } from '../../../../automation/types.ts';
import { AutocompletePopover } from '../../autocomplete/AutocompletePopover.vue';
import { asString } from '../graph.ts';
import { t } from '../../../i18n.ts';
import type { NodeConfigFormProps } from '../NodeConfigForm.vue';
import { formatEditorValue } from './config-value.ts';

/** Script editor with live analysis: completions, diagnostics, and hover cards. */
export const ScriptConfigForm = defineVueComponent<NodeConfigFormProps>(
  ['locale', 'node', 'analysis', 'eventType', 'lastEvent', 'onChange', 'onAnalyzeScript', 'onOpenMediaPicker'],
  (props) => {
  const source = computed(() => asString(props.node.config.source));
  const cursor = ref(source.value.length);
  const completionIndex = ref(0);
  const completionOpen = ref(true);
  const textareaRef = ref<HTMLTextAreaElement | null>(null);
  const completionAnchorRef = ref<HTMLDivElement | null>(null);
  const completionKey = computed(() => props.analysis?.completions.map((completion) => `${completion.label}:${completion.detail ?? ''}`).join('|') ?? '');
  const visibleCompletions = computed(() => props.analysis?.completions.slice(0, 12) ?? []);

  watch(() => [props.node.id, source.value], () => { cursor.value = source.value.length; });
  watch(completionKey, () => {
    completionIndex.value = 0;
    completionOpen.value = visibleCompletions.value.length > 0;
  });

  const change = (nextSource: string, nextCursor: number): void => {
    cursor.value = nextCursor;
    props.onChange({ ...props.node.config, source: nextSource });
    props.onAnalyzeScript(props.node.id, nextSource, nextCursor, props.eventType);
  };

  const applyCompletion = (completion: AutomationScriptCompletion): void => {
    const textarea = textareaRef.value;
    const offset = textarea?.selectionStart ?? cursor.value;
    const before = source.value.slice(0, offset);
    const match = before.match(/[A-Za-z0-9_$]*$/);
    const start = offset - (match?.[0]?.length ?? 0);
    const nextSource = `${source.value.slice(0, start)}${completion.label}${source.value.slice(offset)}`;
    const nextOffset = start + completion.label.length;
    change(nextSource, nextOffset);
    completionOpen.value = false;
    requestAnimationFrame(() => {
      textareaRef.value?.focus();
      textareaRef.value?.setSelectionRange(nextOffset, nextOffset);
    });
  };

  const handleCompletionKeydown = (event: KeyboardEvent): void => {
    const completions = visibleCompletions.value;
    if (!completionOpen.value || completions.length === 0) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      completionIndex.value = (completionIndex.value + 1) % completions.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      completionIndex.value = (completionIndex.value - 1 + completions.length) % completions.length;
    } else if (event.key === 'Tab' || event.key === 'Enter') {
      event.preventDefault();
      const selected = completions[completionIndex.value];
      if (selected) applyCompletion(selected);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      completionOpen.value = false;
    }
  };

  return () => (
    <div class="node-editor-form-stack">
      <div class="node-editor-script-context">
        <span>{t(props.locale, 'scriptEditor')}</span>
        <small>{props.lastEvent ? `${t(props.locale, 'lastEventContext')}: ${props.lastEvent.type}` : t(props.locale, 'noLastEventContext')}</small>
      </div>
      <div class={`plg-float ${source.value.trim().length > 0 ? 'is-filled' : ''}`}>
        <div class="plg-float__control plg-float__control--textarea">
          <div ref={completionAnchorRef} class="node-editor-script-editor" style={{ flex: 1, display: 'flex' }}>
            <textarea
              ref={textareaRef}
              class="node-editor-form-textarea node-editor-form-textarea--code"
              style={{ border: 'none', background: 'transparent', flex: 1 }}
              value={source.value}
              rows={12}
              spellcheck={false}
              placeholder=" "
              aria-label={t(props.locale, 'scriptEditor')}
              onFocus={() => { completionOpen.value = true; }}
              onKeydown={handleCompletionKeydown}
              onInput={(event) => {
                const target = event.currentTarget as HTMLTextAreaElement;
                change(target.value, target.selectionStart ?? target.value.length);
              }}
              onKeyup={(event) => {
                if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Tab' || event.key === 'Enter' || event.key === 'Escape') return;
                const target = event.currentTarget as HTMLTextAreaElement;
                cursor.value = target.selectionStart ?? source.value.length;
                props.onAnalyzeScript(props.node.id, source.value, target.selectionStart ?? source.value.length, props.eventType);
              }}
              onClick={(event) => {
                const target = event.currentTarget as HTMLTextAreaElement;
                cursor.value = target.selectionStart ?? source.value.length;
                props.onAnalyzeScript(props.node.id, source.value, target.selectionStart ?? source.value.length, props.eventType);
              }}
            />
            <AutocompletePopover anchor={completionAnchorRef.value} open={completionOpen.value && visibleCompletions.value.length > 0} updateKey={cursor.value}>
              <div class="node-editor-code-completions" role="listbox">
                {visibleCompletions.value.map((completion, index) => (
                  <button
                    key={`${completion.kind}:${completion.label}`}
                    type="button"
                    role="option"
                    tabindex={-1}
                    aria-selected={index === completionIndex.value}
                    class={index === completionIndex.value ? 'is-selected' : ''}
                    title={completion.documentation ?? completion.detail ?? completion.label}
                    onPointerdown={(event) => event.preventDefault()}
                    onMousedown={(event) => event.preventDefault()}
                    onMouseenter={() => { completionIndex.value = index; }}
                    onClick={() => applyCompletion(completion)}
                  >
                    <strong>{completion.label}</strong>
                    <span>{completion.detail ?? completion.kind}</span>
                    {completion.valueSource === 'live-event' && completion.value !== undefined ? <code>{formatEditorValue(completion.value)}</code> : null}
                  </button>
                ))}
                <small>↑ ↓ {t(props.locale, 'navigate')} · Tab {t(props.locale, 'insertAction')}</small>
              </div>
            </AutocompletePopover>
          </div>
          <label class="plg-float__label">
            {t(props.locale, 'scriptEditor')}
          </label>
        </div>
      </div>
      {props.analysis?.diagnostics.length ? (
        <div class="node-editor-diagnostics" role="status">
          {props.analysis.diagnostics.map((diagnostic, index) => <div key={`${diagnostic.line}:${diagnostic.column}:${index}`} class="node-editor-diagnostic"><span>{diagnostic.line}:{diagnostic.column}</span> {diagnostic.message}</div>)}
        </div>
      ) : null}
      {props.analysis?.hover ? (
        <div class="node-editor-hover-card" role="status">
          <strong>{props.analysis.hover.detail}</strong>
          <span>{props.analysis.hover.documentation}</span>
          {props.analysis.hover.valueSource === 'live-event' && props.analysis.hover.value !== undefined
            ? props.analysis.hover.path === 'event.data'
              ? <pre>{formatEditorValue(props.analysis.hover.value, true)}</pre>
              : <code>{formatEditorValue(props.analysis.hover.value)}</code>
            : null}
        </div>
      ) : null}
    </div>
  );
  },
);

export default ScriptConfigForm;
</script>
