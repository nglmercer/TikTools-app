<script lang="tsx">
import type {
  AutomationEvent,
  AutomationEventType,
  AutomationScriptAnalysis,
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
import { WORKFLOW_EVENT_CHOICES } from './WorkflowWizardModal.vue';
import { asNumber, asString } from './graph.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { SchemaForm } from '../ui/SchemaForm.vue';
import { MediaField } from '../ui/MediaField.vue';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { ScriptConfigForm } from './config/ScriptConfigForm.vue';
import { formatValue, isJsonObject, parseValue } from './config/config-value.ts';

export type NodeConfigFormProps = {
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

function GenericConfigForm({ locale, node, definition, onChange, onOpenMediaPicker }: { locale: Locale; node: WorkflowNode; definition?: NodeDefinition; onChange: (config: JsonObject) => void; onOpenMediaPicker?: OpenMediaPicker }) {
  if (!definition || !isJsonObject(definition.configSchema)) return <GenericConfigFormNoForm locale={locale} />;
  const properties = definition.configSchema.properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties) || Object.keys(properties).length === 0) return <GenericConfigFormNoForm locale={locale} />;
  return <SchemaForm locale={locale} schema={definition.configSchema} value={node.config} onChange={onChange} onOpenMediaPicker={onOpenMediaPicker} />;
}

function GenericConfigFormNoForm({ locale }: { locale: Locale }) {
  return <p class="node-editor-form-empty">{t(locale, 'nodeNoForm')}</p>;
}

export default NodeConfigForm;
</script>
