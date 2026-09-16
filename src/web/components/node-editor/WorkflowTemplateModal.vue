<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type {
  JsonObject,
  NodeDefinition,
  WorkflowGraph,
} from '../../../automation/types.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { Icon } from '../icons/Icon.vue';
import { IconArrowDown, IconChevronLeft } from '../icons/index.ts';
import { presentationForEvent } from '../icons/event-icons.ts';
import { Button } from '../ui/Button.vue';
import { Checkbox } from '../ui/Checkbox.vue';
import { FormField } from '../ui/FormField.vue';
import { MediaField } from '../ui/MediaField.vue';
import { Modal, ModalActions } from '../ui/Modal.vue';
import { NumberInput } from '../ui/NumberInput.vue';
import { PasswordInput } from '../ui/PasswordInput.vue';
import { SearchInput, TextInput } from '../ui/TextInput.vue';
import { Select } from '../ui/Select.vue';
import { i18nText, t, type Locale } from '../../i18n.ts';
import {
  CHAT_TTS_DEFAULTS,
  CHAT_TTS_LOCAL_PRESET,
  filterWorkflowTemplates,
  friendlyNodeType,
  isHttpUrl,
  missingTemplateNodes,
  NODE_TYPES,
  toChatTtsOptions,
  WORKFLOW_TEMPLATES,
  workflowTemplateAvailable,
  workflowTemplateById,
  type ChatTtsTemplateOptions,
  type WorkflowTemplate,
} from './workflow-templates.ts';

type WorkflowTemplateModalProps = {
  locale: Locale;
  definitions: NodeDefinition[];
  onClose: () => void;
  onCreate: (graph: WorkflowGraph) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

export const WorkflowTemplateModal = defineVueComponent<WorkflowTemplateModalProps>(
  ['locale', 'definitions', 'onClose', 'onCreate', 'onOpenMediaPicker'],
  (props) => {
  const query = ref('');
  const selectedId = ref<string | null>(null);
  const name = ref('');
  const error = ref('');
  const tts = ref<ChatTtsTemplateOptions>({ ...CHAT_TTS_DEFAULTS });
  const ttsPreset = ref<'local' | 'custom'>('local');
  const webhookUrl = ref('https://');
  const filePath = ref('');
  const delta = ref<number | null>(1);

  const selectTemplate = (template: WorkflowTemplate): void => {
    selectedId.value = template.id;
    name.value = i18nText(props.locale, template.title);
    error.value = '';
    tts.value = { ...CHAT_TTS_DEFAULTS };
    ttsPreset.value = 'local';
    webhookUrl.value = 'https://';
    filePath.value = '';
    delta.value = 1;
  };

  const backToList = (): void => {
    selectedId.value = null;
    error.value = '';
  };

  const templateOptions = (template: WorkflowTemplate): JsonObject => {
    switch (template.id) {
      case 'chat-tts':
        return { ...toChatTtsOptions({ ...tts.value }) };
      case 'chat-webhook':
      case 'gift-webhook':
        return { url: webhookUrl.value.trim() };
      case 'gift-sound':
      case 'follow-sound':
        return { filePath: filePath.value };
      case 'chat-points':
        return { delta: delta.value ?? 1 };
      default:
        return {};
    }
  };

  const validateTemplate = (template: WorkflowTemplate): string => {
    if (!name.value.trim()) return t(props.locale, 'workflowNameRequired');
    switch (template.id) {
      case 'chat-tts': {
        const options = toChatTtsOptions({ ...tts.value });
        if (!isHttpUrl(options.serverUrl)) return t(props.locale, 'invalidUrl');
        if (!options.voice || !options.language) return t(props.locale, 'dialogRequired');
        return '';
      }
      case 'chat-webhook':
      case 'gift-webhook':
        return isHttpUrl(webhookUrl.value) ? '' : t(props.locale, 'invalidUrl');
      case 'gift-sound':
      case 'follow-sound':
        return filePath.value.trim() ? '' : t(props.locale, 'dialogRequired');
      default:
        return '';
    }
  };

  const create = (): void => {
    const template = selectedId.value ? workflowTemplateById(selectedId.value) : undefined;
    if (!template) return;
    const validation = validateTemplate(template);
    if (validation) {
      error.value = validation;
      return;
    }
    try {
      const graph = template.build({ definitions: props.definitions }, { ...templateOptions(template), name: name.value.trim() });
      props.onCreate(graph);
    } catch (thrown) {
      const message = thrown instanceof Error ? thrown.message : String(thrown);
      const missing = /^Required workflow node is unavailable: (.+)$/.exec(message)?.[1];
      error.value = missing
        ? t(props.locale, 'templateMissingNode', { name: friendlyNodeType(missing) })
        : message;
    }
  };

  return () => {
    const { locale, definitions, onClose, onOpenMediaPicker } = props;
    const template = selectedId.value ? workflowTemplateById(selectedId.value) : undefined;
    const visible = filterWorkflowTemplates(WORKFLOW_TEMPLATES, query.value);

    if (!template) {
      return (
        <Modal
          title={t(locale, 'quickTemplates')}
          description={t(locale, 'templatesLead')}
          size="lg"
          onClose={onClose}
        >
          <div class="template-list">
            <SearchInput
              value={query.value}
              onValueChange={(next) => { query.value = next; }}
              placeholder={t(locale, 'searchTemplates')}
              name="template-search"
            />
            <div class="template-grid" role="group" aria-label={t(locale, 'quickTemplates')}>
              {visible.map((entry) => {
                const available = workflowTemplateAvailable(entry, definitions);
                const missing = missingTemplateNodes(entry, definitions);
                return (
                  <button
                    key={entry.id}
                    type="button"
                    class={`template-card${available ? '' : ' is-disabled'}`}
                    disabled={!available}
                    onClick={() => selectTemplate(entry)}
                    aria-label={i18nText(locale, entry.title)}
                  >
                    <span class="template-card__icon" aria-hidden="true"><Icon name={entry.icon} size={20} /></span>
                    <span class="template-card__text">
                      <strong>{i18nText(locale, entry.title)}</strong>
                      <small>{i18nText(locale, entry.description)}</small>
                      {!available && (
                        <small class="template-card__missing">
                          {t(locale, 'templateRequires', { name: missing.map(friendlyNodeType).join(', ') })}
                        </small>
                      )}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        </Modal>
      );
    }

    const actionType = template.requiredNodeTypes.find((type) => type !== NODE_TYPES.triggerEvent);
    const actionTitle = actionType
      ? (definitions.find((definition) => definition.type === actionType)?.title ?? friendlyNodeType(actionType))
      : '';
    const trigger = presentationForEvent(template.eventType);

    return (
      <Modal
        title={i18nText(locale, template.title)}
        description={i18nText(locale, template.description)}
        size="lg"
        onClose={onClose}
        footer={
          <ModalActions>
            <Button variant="soft" icon={<IconChevronLeft size={14} />} onClick={backToList}>{t(locale, 'backToTemplates')}</Button>
            <Button variant="primary" onClick={create}>{t(locale, 'createWorkflow')}</Button>
          </ModalActions>
        }
      >
        <div class="template-config">
          <div class="template-preview" aria-label={t(locale, 'templateReviewHint')}>
            <span class="template-preview__row">
              <Icon name={trigger.icon} size={16} />
              <span>{i18nText(locale, trigger.label)}</span>
            </span>
            <span class="template-preview__arrow" aria-hidden="true"><IconArrowDown size={14} /></span>
            <span class="template-preview__row">
              <Icon name={template.icon} size={16} />
              <span>{actionTitle}</span>
            </span>
          </div>

          <FormField label={t(locale, 'workflowName')} required>
            <TextInput
              value={name.value}
              onValueChange={(next) => { name.value = next; }}
              placeholder={t(locale, 'workflowNamePlaceholder')}
              name="template-name"
              required
            />
          </FormField>

          {template.id === 'chat-tts' && (
            <ChatTtsOptionsForm
              locale={locale}
              options={tts.value}
              preset={ttsPreset.value}
              onPresetChange={(preset) => {
                ttsPreset.value = preset;
                if (preset === 'local') tts.value = { ...tts.value, serverUrl: CHAT_TTS_LOCAL_PRESET };
              }}
              onChange={(next) => {
                tts.value = next;
                ttsPreset.value = next.serverUrl.trim() === CHAT_TTS_LOCAL_PRESET ? 'local' : 'custom';
              }}
            />
          )}

          {(template.id === 'chat-webhook' || template.id === 'gift-webhook') && (
            <FormField label={t(locale, 'webhookUrl')} required>
              <TextInput
                value={webhookUrl.value}
                onValueChange={(next) => { webhookUrl.value = next; }}
                placeholder="https://"
                name="template-url"
                spellCheck={false}
                required
              />
            </FormField>
          )}

          {(template.id === 'gift-sound' || template.id === 'follow-sound') && (
            <MediaField
              locale={locale}
              label={t(locale, 'nodeFilePath')}
              hint={t(locale, 'nodeFilePathHint')}
              value={filePath.value}
              onValueChange={(next) => { filePath.value = typeof next === 'string' ? next : ''; }}
              onOpenMediaPicker={onOpenMediaPicker}
              name="template-file"
            />
          )}

          {template.id === 'chat-points' && (
            <NumberInput
              label={t(locale, 'nodeDelta')}
              value={delta.value}
              onValueChange={(next) => { delta.value = next; }}
              step={1}
              name="template-delta"
            />
          )}

          {error.value ? <p class="template-error" role="alert">{error.value}</p> : null}
          <p class="template-hint">{t(locale, 'templateReviewHint')}</p>
        </div>
      </Modal>
    );
  };
  },
);

function ChatTtsOptionsForm({
  locale,
  options,
  preset,
  onPresetChange,
  onChange,
}: {
  locale: Locale;
  options: ChatTtsTemplateOptions;
  preset: 'local' | 'custom';
  onPresetChange: (preset: 'local' | 'custom') => void;
  onChange: (options: ChatTtsTemplateOptions) => void;
}) {
  const patch = (delta: Partial<ChatTtsTemplateOptions>): void => onChange({ ...options, ...delta });
  return (
    <div class="template-options">
      <Select
        label={t(locale, 'serverUrl')}
        value={preset}
        options={[
          { value: 'local', label: t(locale, 'ttsPresetLocal') },
          { value: 'custom', label: t(locale, 'ttsPresetCustom') },
        ]}
        onValueChange={(next) => onPresetChange(next === 'custom' ? 'custom' : 'local')}
        name="tts-preset"
      />
      <FormField label={t(locale, 'serverUrl')} required>
        <TextInput
          value={options.serverUrl}
          onValueChange={(next) => patch({ serverUrl: next })}
          placeholder="http://localhost:3000"
          name="tts-server"
          spellCheck={false}
          required
        />
      </FormField>
      <FormField label={t(locale, 'apiToken')} hint={t(locale, 'apiTokenStored')}>
        <PasswordInput
          value={options.apiToken}
          onValueChange={(next) => patch({ apiToken: next })}
          name="tts-token"
          autoComplete="off"
          clearable
        />
      </FormField>
      <div class="template-row">
        <FormField label={t(locale, 'nodeVoice')} required>
          <TextInput
            value={options.voice}
            onValueChange={(next) => patch({ voice: next })}
            name="tts-voice"
            required
          />
        </FormField>
        <FormField label={t(locale, 'nodeLanguage')} required>
          <TextInput
            value={options.language}
            onValueChange={(next) => patch({ language: next })}
            name="tts-language"
            required
          />
        </FormField>
      </div>
      <Checkbox
        checked={options.playNow}
        onCheckedChange={(next) => patch({ playNow: next })}
        label={t(locale, 'playNow')}
        name="tts-play-now"
      />
      <Select
        label={t(locale, 'textSource')}
        value={options.textSource}
        options={[
          { value: 'raw', label: t(locale, 'rawChat') },
          { value: 'textintel', label: t(locale, 'textIntelligence') },
        ]}
        onValueChange={(next) => patch({ textSource: next === 'textintel' ? 'textintel' : 'raw' })}
        name="tts-text-source"
      />
    </div>
  );
}

export default WorkflowTemplateModal;
</script>
