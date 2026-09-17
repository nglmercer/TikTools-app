<script lang="tsx">
import { computed, ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type {
  AutomationEvent,
  AutomationEventType,
  AutomationScriptAnalysis,
  AutomationScriptCompletion,
  JsonObject,
  JsonValue,
  NodeDefinition,
  WorkflowNode,
} from '../../../automation/types.ts';
import { TextInput } from '../ui/TextInput.vue';
import { NumberInput } from '../ui/NumberInput.vue';
import { Select } from '../ui/Select.vue';
import { TemplateField } from '../ui/fields/TemplateField.vue';
import { getTemplateSuggestions, type TemplateSuggestionScope } from './template-suggestions.ts';
import { HttpRequestEditor } from '../http/index.ts';
import { AutocompletePopover } from '../autocomplete/AutocompletePopover.vue';
import { WORKFLOW_EVENT_CHOICES } from './WorkflowWizardModal.vue';
import { asNumber, asString } from './graph.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { SchemaForm } from '../ui/SchemaForm.vue';
import { MediaField } from '../ui/MediaField.vue';
import type { OpenMediaPicker } from '../../../shared/messages.ts';

type NodeConfigFormProps = {
  locale: Locale;
  node: WorkflowNode;
  definition?: NodeDefinition;
  analysis?: AutomationScriptAnalysis;
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  onChange: (config: JsonObject) => void;
  onAnalyzeScript: (nodeId: string, source: string, offset: number, eventType?: AutomationEventType) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

export function NodeConfigForm({ locale, node, definition, analysis, eventType, lastEvent, onChange, onAnalyzeScript, onOpenMediaPicker }: NodeConfigFormProps) {
  const update = (key: string, value: JsonValue): void => onChange({ ...node.config, [key]: value });
  const config = node.config;
  const templateValues = (scope: TemplateSuggestionScope = 'message') => getTemplateSuggestions(eventType, locale, lastEvent, scope);

  if (!definition) {
    return <GenericConfigForm locale={locale} node={node} onChange={onChange} onOpenMediaPicker={onOpenMediaPicker} />;
  }

  switch (node.type) {
    case 'trigger.event':
      return (
        <div class="node-editor-form-stack">
          <Select
            label={t(locale, 'nodeEventType')}
            hint={t(locale, 'nodeEventTypeHint')}
            value={asString(config.eventType, 'tiktok.chat')}
            options={WORKFLOW_EVENT_CHOICES.map((choice) => ({ value: choice.value, label: i18nText(locale, choice.label) }))}
            onValueChange={(value) => update('eventType', value)}
          />
        </div>
      );
    case 'condition.compare':
      return (
        <div class="node-editor-form-stack">
          <TemplateField locale={locale}
            label={t(locale, 'nodeValuePath')}
            hint={t(locale, 'nodeValuePathHint')}
            value={asString(config.leftPath)}
            onValueChange={(value) => update('leftPath', value)}
            suggestions={templateValues('compare')}
            mode="options"
          />
          <Select
            label={t(locale, 'nodeOperator')}
            value={asString(config.operator, 'equals')}
            options={[
              ['equals', t(locale, 'nodeEquals')],
              ['not-equals', t(locale, 'nodeNotEquals')],
              ['greater-than', t(locale, 'nodeGreaterThan')],
              ['greater-or-equal', t(locale, 'nodeGreaterOrEqual')],
              ['less-than', t(locale, 'nodeLessThan')],
              ['less-or-equal', t(locale, 'nodeLessOrEqual')],
              ['contains', t(locale, 'nodeContains')],
              ['starts-with', t(locale, 'nodeStartsWith')],
              ['truthy', t(locale, 'nodeTruthy')],
              ['falsy', t(locale, 'nodeFalsy')],
            ].map(([value, label]) => ({ value: value ?? '', label: label ?? '' }))}
            onValueChange={(value) => update('operator', value)}
          />
          <TextInput label={t(locale, 'nodeCompareWith')} hint={t(locale, 'nodeCompareWithHint')} value={formatValue(config.right)} onValueChange={(value) => update('right', parseValue(value))} />
        </div>
      );
    case 'transform.template':
      return (
        <div class="node-editor-form-stack">
          <TemplateField locale={locale} label={t(locale, 'nodeTemplate')} hint={t(locale, 'nodeTemplateHint')} value={asString(config.template)} onValueChange={(value) => update('template', value)} suggestions={templateValues('message')} scope="message" multiline rows={5} />
        </div>
      );
    case 'transform.script':
      return <ScriptConfigForm locale={locale} node={node} analysis={analysis} eventType={eventType} lastEvent={lastEvent} onChange={onChange} onAnalyzeScript={onAnalyzeScript} />;
    case 'control.delay':
      return (
        <div class="node-editor-form-stack">
          <NumberInput label={t(locale, 'nodeDelay')} hint={t(locale, 'nodeDelayHint')} value={asNumber(config.delayMs)} min={0} max={3_600_000} step={100} suffix="ms" onValueChange={(value) => update('delayMs', value)} />
        </div>
      );
    case 'control.cooldown':
      return (
        <div class="node-editor-form-stack">
          <NumberInput label={t(locale, 'nodeDuration')} hint={t(locale, 'nodeCooldownHint')} value={asNumber(config.durationMs)} min={0} max={86_400_000} step={100} suffix="ms" onValueChange={(value) => update('durationMs', value)} />
          <TemplateField locale={locale} label={t(locale, 'nodeCooldownKey')} value={asString(config.key)} onValueChange={(value) => update('key', value)} suggestions={templateValues('identity')} scope="identity" />
        </div>
      );
    case 'action.log':
      return (
        <div class="node-editor-form-stack">
          <TemplateField locale={locale} label={t(locale, 'nodeMessage')} hint={t(locale, 'nodeTemplateHint')} value={asString(config.message)} onValueChange={(value) => update('message', value)} suggestions={templateValues('message')} scope="message" multiline rows={5} />
        </div>
      );
    case 'action.http':
      return (
        <HttpRequestEditor
          locale={locale}
          config={config}
          onPatchConfig={(patch) => onChange({ ...config, ...patch })}
          methodOptions={['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((value) => ({ value, label: value }))}
          defaultMethod="GET"
          methodLabel={t(locale, 'nodeMethod')}
          urlLabel={t(locale, 'nodeUrl')}
          urlSuggestions={templateValues('http-url')}
          bodyLabel={t(locale, 'nodeRequestBody')}
          bodySuggestions={templateValues('http-data')}
          defaultBodyMode="json"
          headersLabel={t(locale, 'nodeHeaders')}
          headersHint={t(locale, 'nodeHeadersHint')}
          headerSuggestions={templateValues('http-data')}
          timeoutLabel={t(locale, 'nodeTimeout')}
          allowPrivateLabel={t(locale, 'nodeAllowPrivateNetwork')}
          showResponseType
          responseTypeLabel={t(locale, 'nodeResponseType')}
          showRedirect
          redirectLabel={t(locale, 'nodeRedirect')}
          redirectFollowLabel={t(locale, 'nodeFollowRedirects')}
          redirectBlockLabel={t(locale, 'nodeBlockRedirects')}
        />
      );
    case 'action.play-sound':
      return (
        <div class="node-editor-form-stack">
          <MediaField locale={locale} label={t(locale, 'nodeFilePath')} hint={t(locale, 'nodeFilePathHint')} value={config.filePath} onValueChange={(value) => update('filePath', value)} onOpenMediaPicker={onOpenMediaPicker} name="filePath" />
          <NumberInput label={t(locale, 'nodeVolume')} value={asNumber(config.volume, 1)} min={0} max={1} step={0.05} onValueChange={(value) => update('volume', value)} />
          <Select label={t(locale, 'nodeOverlap')} value={asString(config.overlap, 'allow')} options={[{ value: 'allow', label: t(locale, 'nodeAllowOverlap') }, { value: 'restart', label: t(locale, 'nodeRestartOverlap') }, { value: 'drop', label: t(locale, 'nodeDropOverlap') }]} onValueChange={(value) => update('overlap', value)} />
        </div>
      );
    // Legacy quarantine: the host catalog currently exposes no `action.tts`
    // node, so this branch is unreachable from the node picker. It stays so
    // older graphs keep rendering, and so future catalog support lights up
    // without new UI work. Do not build new features on this specialization.
    case 'action.tts':
      return (
        <div class="node-editor-form-stack">
          <TemplateField locale={locale} label={t(locale, 'nodeText')} hint={t(locale, 'nodeTemplateHint')} value={asString(config.text)} onValueChange={(value) => update('text', value)} suggestions={templateValues('text')} multiline rows={5} />
          <TextInput label={t(locale, 'nodeVoice')} value={asString(config.voice, 'M1')} onValueChange={(value) => update('voice', value)} />
          <Select label={t(locale, 'nodeLanguage')} value={asString(config.lang, 'en')} options={[{ value: 'en', label: 'English' }, { value: 'es', label: 'Español' }]} onValueChange={(value) => update('lang', value)} />
          <Select label={t(locale, 'nodeAudioFormat')} value={asString(config.format, 'wav')} options={[{ value: 'wav', label: 'WAV' }, { value: 'ogg', label: 'OGG' }]} onValueChange={(value) => update('format', value)} />
        </div>
      );
    case 'action.adjust-points':
      return (
        <div class="node-editor-form-stack">
          <TemplateField locale={locale} label={t(locale, 'nodeViewer')} hint={t(locale, 'nodeViewerHint')} value={asString(config.uniqueId)} onValueChange={(value) => update('uniqueId', value)} suggestions={templateValues('identity')} scope="identity" />
          <NumberInput label={t(locale, 'nodeDelta')} value={asNumber(config.delta, 10)} step={1} onValueChange={(value) => update('delta', value)} />
        </div>
      );
    default:
      return <GenericConfigForm locale={locale} node={node} definition={definition} onChange={onChange} onOpenMediaPicker={onOpenMediaPicker} />;
  }
}

const ScriptConfigForm = defineVueComponent<NodeConfigFormProps>(
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
                    aria-selected={index === completionIndex.value}
                    class={index === completionIndex.value ? 'is-selected' : ''}
                    title={completion.documentation ?? completion.detail ?? completion.label}
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

function GenericConfigForm({ locale, node, definition, onChange, onOpenMediaPicker }: { locale: Locale; node: WorkflowNode; definition?: NodeDefinition; onChange: (config: JsonObject) => void; onOpenMediaPicker?: OpenMediaPicker }) {
  if (!definition || !isJsonObject(definition.configSchema)) return <GenericConfigFormNoForm locale={locale} />;
  const properties = definition.configSchema.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties) || Object.keys(properties).length === 0) return <GenericConfigFormNoForm locale={locale} />;
  return <SchemaForm locale={locale} schema={definition.configSchema} value={node.config} onChange={onChange} onOpenMediaPicker={onOpenMediaPicker} />;
}

function formatValue(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'object') return '';
  return String(value);
}

function formatEditorValue(value: JsonValue, pretty = false): string {
  if (typeof value === 'string') return JSON.stringify(value);
  if (value === null) return 'null';
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    const serialized = JSON.stringify(value, pretty ? null : undefined, pretty ? 2 : undefined) ?? String(value);
    if (pretty) return serialized.length > 8_000 ? `${serialized.slice(0, 7_997)}...` : serialized;
    return serialized.length > 140 ? `${serialized.slice(0, 137)}...` : serialized;
  } catch {
    return String(value);
  }
}

function parseValue(value: string): JsonValue {
  const trimmed = value.trim();
  if (!trimmed) return '';
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const number = Number(trimmed);
  return Number.isFinite(number) && trimmed !== '' ? number : value;
}

function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function GenericConfigFormNoForm({ locale }: { locale: Locale }) {
  return <p class="node-editor-form-empty">{t(locale, 'nodeNoForm')}</p>;
}

export default NodeConfigForm;
</script>
