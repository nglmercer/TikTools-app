<script lang="tsx">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { JsonObject } from '../../../automation/types.ts';
import { BEHAVIOR_TRIGGERS } from '../../../automation/behavior/schema.ts';
import type {
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
} from '../../../automation/behavior/types.ts';
import { Icon } from '../../components/icons/Icon.vue';
import { IconArrowDown, IconChevronLeft } from '../../components/icons/index.ts';
import { presentationForEvent } from '../../components/icons/event-icons.ts';
import { Button } from '../../components/ui/Button.vue';
import { FormField } from '../../components/ui/FormField.vue';
import { Modal, ModalActions } from '../../components/ui/Modal.vue';
import { SchemaForm } from '../../components/ui/SchemaForm.vue';
import { SearchInput, TextInput } from '../../components/ui/TextInput.vue';
import { isHttpUrl } from '../../components/node-editor/workflow-templates.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { availableActionTypes, triggerLabel } from './helpers.vue';
import {
  applyRuleTemplate,
  BUILTIN_RULE_TEMPLATES,
  exportRuleTemplate,
  missingRuleTemplateRequirements,
  ruleParamDefaults,
  type RuleTemplate,
} from './rule-templates.ts';
import { parseRuleTemplateImport } from '../../features/rule-templates.ts';

type RuleTemplateModalProps = {
  locale: Locale;
  snapshot: BehaviorSnapshot;
  customTemplates: RuleTemplate[];
  customError?: string | null;
  onClose: () => void;
  onApply: (actions: LiveAction[], event: LiveEvent) => void;
  onImport: (templates: RuleTemplate[]) => Promise<number>;
  onDeleteCustom: (id: string) => void;
};

type Screen =
  | { kind: 'list' }
  | { kind: 'configure'; template: RuleTemplate }
  | { kind: 'import' };

function downloadTemplate(template: RuleTemplate): void {
  const blob = new Blob([exportRuleTemplate(template)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${template.id}.tiktemplate.json`;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

export const RuleTemplateModal = defineVueComponent<RuleTemplateModalProps>(
  ['locale', 'snapshot', 'customTemplates', 'customError', 'onClose', 'onApply', 'onImport', 'onDeleteCustom'],
  (props) => {
    const screen = ref<Screen>({ kind: 'list' });
    const query = ref('');
    const eventName = ref('');
    const actionNames = ref<string[]>([]);
    const params = ref<JsonObject>({});
    const error = ref('');
    const staged = ref<{ source: string; templates: RuleTemplate[] } | null>(null);
    const importErrors = ref<string[]>([]);
    const importAdded = ref(0);
    const importing = ref(false);
    const dragActive = ref(false);
    const fileInput = ref<HTMLInputElement | null>(null);
    const dropzone = ref<HTMLDivElement | null>(null);
    const pasteHint = ref('');

    const allTemplates = computed<RuleTemplate[]>(() => [...BUILTIN_RULE_TEMPLATES, ...props.customTemplates]);
    const availableTypes = computed(() => availableActionTypes(props.snapshot.plugins, props.snapshot.actionTypes));
    const knownTriggers = computed<string[]>(() => [
      ...BEHAVIOR_TRIGGERS,
      ...(props.snapshot.eventTypes ?? []).map((entry) => entry.type),
    ]);

    const visible = computed<RuleTemplate[]>(() => {
      const needle = query.value.trim().toLowerCase();
      if (!needle) return allTemplates.value;
      return allTemplates.value.filter((template) =>
        i18nText(props.locale, template.title).toLowerCase().includes(needle)
        || i18nText(props.locale, template.description).toLowerCase().includes(needle)
        || template.event.trigger.toLowerCase().includes(needle),
      );
    });

    const requirementsOf = (template: RuleTemplate) =>
      missingRuleTemplateRequirements(template, availableTypes.value, knownTriggers.value);

    const selectTemplate = (template: RuleTemplate): void => {
      screen.value = { kind: 'configure', template };
      eventName.value = template.event.name;
      actionNames.value = template.actions.map((action) => action.name);
      params.value = ruleParamDefaults(template.params);
      error.value = '';
    };

    const requiredParams = (template: RuleTemplate): string[] => {
      const required = template.params?.required;
      return Array.isArray(required) ? required.filter((entry): entry is string => typeof entry === 'string') : [];
    };

    const validateConfigure = (template: RuleTemplate): string => {
      if (!eventName.value.trim()) return t(props.locale, 'dialogRequired');
      if (actionNames.value.some((name) => !name.trim())) return t(props.locale, 'dialogRequired');
      for (const key of requiredParams(template)) {
        const value = params.value[key];
        if (value === undefined || value === null || (typeof value === 'string' && value.trim() === '')) {
          return t(props.locale, 'dialogRequired');
        }
      }
      if ((template.id === 'gift-webhook' || template.id === 'chat-webhook') && !isHttpUrl(String(params.value['url'] ?? ''))) {
        return t(props.locale, 'invalidUrl');
      }
      return '';
    };

    const apply = (template: RuleTemplate): void => {
      const validation = validateConfigure(template);
      if (validation) {
        error.value = validation;
        return;
      }
      const applied = applyRuleTemplate(template, { ...params.value }, {
        eventName: eventName.value,
        actionNames: actionNames.value,
      });
      props.onApply(applied.actions, applied.event);
      props.onClose();
    };

    /** Stages dropped, picked, or pasted content: parses immediately and shows
     * what would be imported. Raw JSON is never typed into the UI. */
    const stageContent = (text: string, source: string): void => {
      importAdded.value = 0;
      pasteHint.value = '';
      const { templates, errors } = parseRuleTemplateImport(text);
      if (templates.length === 0 && errors.length === 0) {
        staged.value = null;
        importErrors.value = [t(props.locale, 'behavior.copy.tplImportEmpty')];
        return;
      }
      staged.value = templates.length > 0 ? { source, templates } : null;
      importErrors.value = errors;
    };

    const readImportFile = (file: File): void => {
      importErrors.value = [];
      const reader = new FileReader();
      reader.onload = () => {
        stageContent(typeof reader.result === 'string' ? reader.result : '', file.name);
      };
      reader.onerror = () => {
        staged.value = null;
        importErrors.value = [t(props.locale, 'behavior.copy.tplImportUnreadable')];
      };
      reader.readAsText(file);
    };

    const openFilePicker = (): void => {
      fileInput.value?.click();
    };

    // Paste events need no clipboard permission, unlike readText(), which the
    // desktop webview denies. The document listener covers Ctrl+V anywhere on
    // the import screen (it holds no text inputs, so nothing is hijacked).
    const handleDocumentPaste = (event: ClipboardEvent): void => {
      if (screen.value.kind !== 'import') return;
      const text = event.clipboardData?.getData('text');
      if (text) {
        event.preventDefault();
        stageContent(text, t(props.locale, 'behavior.copy.tplClipboardSource'));
      }
    };
    onMounted(() => document.addEventListener('paste', handleDocumentPaste));
    onUnmounted(() => document.removeEventListener('paste', handleDocumentPaste));

    const pasteFromClipboard = async (): Promise<void> => {
      try {
        const read = navigator.clipboard?.readText;
        if (!read) throw new Error('clipboard unavailable');
        stageContent(await read.call(navigator.clipboard), t(props.locale, 'behavior.copy.tplClipboardSource'));
      } catch {
        // Fall back to guided manual paste: focus the dropzone and let the
        // document paste listener stage whatever the user pastes with Ctrl+V.
        pasteHint.value = t(props.locale, 'behavior.copy.tplPastePressKeys');
        dropzone.value?.focus();
      }
    };

    const runImport = async (): Promise<void> => {
      if (importing.value) return;
      const pending = staged.value;
      if (!pending) {
        importErrors.value = [t(props.locale, 'behavior.copy.tplImportEmpty')];
        return;
      }
      importing.value = true;
      try {
        const added = await props.onImport(pending.templates);
        importAdded.value = added;
        if (added > 0) staged.value = null;
      } finally {
        importing.value = false;
      }
    };

    return () => {
      const locale = props.locale;
      const current = screen.value;

      if (current.kind === 'import') {
        return (
          <Modal
            title={t(locale, 'behavior.copy.tplImportTitle')}
            description={t(locale, 'behavior.copy.tplImportLead')}
            size="lg"
            onClose={props.onClose}
            footer={
              <ModalActions>
                <Button variant="soft" icon={<IconChevronLeft size={14} />} onClick={() => { screen.value = { kind: 'list' }; }}>
                  {t(locale, 'behavior.copy.tplBack')}
                </Button>
                <Button variant="primary" loading={importing.value} onClick={() => void runImport()}>
                  {t(locale, 'behavior.copy.tplDoImport')}
                </Button>
              </ModalActions>
            }
          >
            <div class="template-config">
              <div
                ref={dropzone}
                role="group"
                aria-label={t(locale, 'behavior.copy.tplImportTitle')}
                tabindex={0}
                class={`rule-template-dropzone${dragActive.value ? ' is-dragging' : ''}`}
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
                  if (file) readImportFile(file);
                }}
              >
                <Icon name="template" size={24} />
                <p class="rule-template-dropzone__title">{t(locale, 'behavior.copy.tplDropTitle')}</p>
                <p class="rule-template-dropzone__hint">{t(locale, 'behavior.copy.tplDropHint')}</p>
                <div class="rule-template-dropzone__actions">
                  <span onClick={(event) => event.stopPropagation()}>
                    <Button variant="soft" size="md" onClick={() => openFilePicker()}>
                      {t(locale, 'behavior.copy.tplBrowse')}
                    </Button>
                  </span>
                  <span onClick={(event) => event.stopPropagation()}>
                    <Button variant="soft" size="md" onClick={() => void pasteFromClipboard()}>
                      {t(locale, 'behavior.copy.tplPaste')}
                    </Button>
                  </span>
                </div>
                <input
                  ref={fileInput}
                  type="file"
                  accept=".json,application/json"
                  hidden
                  onChange={(event) => {
                    const input = event.target as HTMLInputElement;
                    const file = input.files?.[0];
                    input.value = '';
                    if (file) readImportFile(file);
                  }}
                />
              </div>
              {staged.value && (
                <div class="rule-template-staged" role="status">
                  <div class="rule-template-staged__head">
                    <strong>{t(locale, 'behavior.copy.tplStaged', { count: staged.value.templates.length })}</strong>
                    <span class="rule-template-staged__source">
                      {t(locale, 'behavior.copy.tplStagedFrom', { source: staged.value.source })}
                    </span>
                    <Button variant="ghost" size="sm" onClick={() => { staged.value = null; }}>
                      {t(locale, 'behavior.copy.tplClearStaged')}
                    </Button>
                  </div>
                  <ul>
                    {staged.value.templates.map((template) => (
                      <li key={template.id}>
                        <Icon name={template.icon} size={14} />
                        <span>{i18nText(locale, template.title)}</span>
                        <small>{template.event.trigger}</small>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {pasteHint.value && (
                <p class="rule-template-hint" role="status">{pasteHint.value}</p>
              )}
              {importAdded.value > 0 && (
                <p class="rule-template-success" role="status">
                  {t(locale, 'behavior.copy.tplImported', { count: importAdded.value })}
                </p>
              )}
              {importErrors.value.map((message) => (
                <p class="template-error" role="alert" key={message}>{message}</p>
              ))}
            </div>
          </Modal>
        );
      }

      if (current.kind === 'configure') {
        const template = current.template;
        const trigger = presentationForEvent(template.event.trigger);
        return (
          <Modal
            title={i18nText(locale, template.title)}
            description={i18nText(locale, template.description)}
            size="lg"
            onClose={props.onClose}
            footer={
              <ModalActions>
                <Button variant="soft" icon={<IconChevronLeft size={14} />} onClick={() => { screen.value = { kind: 'list' }; }}>
                  {t(locale, 'behavior.copy.tplBack')}
                </Button>
                {template.custom && (
                  <Button variant="soft" onClick={() => downloadTemplate(template)}>
                    {t(locale, 'behavior.copy.tplExport')}
                  </Button>
                )}
                {template.custom && (
                  <Button
                    variant="danger"
                    onClick={() => {
                      props.onDeleteCustom(template.id);
                      screen.value = { kind: 'list' };
                    }}
                  >
                    {t(locale, 'behavior.copy.tplDelete')}
                  </Button>
                )}
                <Button variant="primary" onClick={() => apply(template)}>
                  {t(locale, 'behavior.copy.tplApply')}
                </Button>
              </ModalActions>
            }
          >
            <div class="template-config">
              <div class="template-preview" aria-label={t(locale, 'behavior.copy.tplReviewHint')}>
                <span class="template-preview__row">
                  <Icon name={trigger.icon} size={16} />
                  <span>{triggerLabel(template.event.trigger, props.snapshot.eventTypes ?? [], locale)}</span>
                </span>
                <span class="template-preview__arrow" aria-hidden="true"><IconArrowDown size={14} /></span>
                {template.actions.map((action) => (
                  <span class="template-preview__row" key={action.typeId}>
                    <Icon name={template.icon} size={16} />
                    <span>{action.typeId}</span>
                  </span>
                ))}
              </div>

              <FormField label={t(locale, 'behavior.copy.tplEventName')} required>
                <TextInput
                  value={eventName.value}
                  onValueChange={(next) => { eventName.value = next; }}
                  name="rule-template-event-name"
                  required
                />
              </FormField>

              {template.actions.map((action, index) => (
                <FormField
                  key={`${action.typeId}-${index}`}
                  label={template.actions.length > 1
                    ? t(locale, 'behavior.copy.tplActionName', { name: action.typeId })
                    : t(locale, 'behavior.copy.tplSingleActionName')}
                  required
                >
                  <TextInput
                    value={actionNames.value[index] ?? ''}
                    onValueChange={(next) => {
                      actionNames.value = actionNames.value.map((entry, position) => position === index ? next : entry);
                    }}
                    name={`rule-template-action-name-${index}`}
                    required
                  />
                </FormField>
              ))}

              {template.params && (
                <FormField label={t(locale, 'behavior.copy.tplParams')}>
                  <SchemaForm
                    locale={locale}
                    schema={template.params}
                    value={params.value}
                    onChange={(next) => { params.value = next; }}
                  />
                </FormField>
              )}

              {error.value ? <p class="template-error" role="alert">{error.value}</p> : null}
              <p class="template-hint">{t(locale, 'behavior.copy.tplReviewHint')}</p>
            </div>
          </Modal>
        );
      }

      return (
        <Modal
          title={t(locale, 'behavior.copy.tplTitle')}
          description={t(locale, 'behavior.copy.tplLead')}
          size="lg"
          onClose={props.onClose}
        >
          <div class="template-list">
            <div class="rule-template-list-row">
              <SearchInput
                value={query.value}
                onValueChange={(next) => { query.value = next; }}
                placeholder={t(locale, 'behavior.copy.tplSearch')}
                name="rule-template-search"
              />
              <Button variant="soft" size="md" onClick={() => { screen.value = { kind: 'import' }; }}>
                {t(locale, 'behavior.copy.tplImport')}
              </Button>
            </div>
            {props.customError ? <p class="template-error" role="alert">{props.customError}</p> : null}
            <div class="template-grid" role="group" aria-label={t(locale, 'behavior.copy.tplTitle')}>
              {visible.value.map((entry) => {
                const requirements = requirementsOf(entry);
                const available = requirements.missingTypeIds.length === 0 && !requirements.unknownTrigger;
                return (
                  <button
                    key={`${entry.custom ? 'custom' : 'builtin'}-${entry.id}`}
                    type="button"
                    class={`template-card${available ? '' : ' is-disabled'}`}
                    disabled={!available}
                    onClick={() => selectTemplate(entry)}
                    aria-label={i18nText(locale, entry.title)}
                  >
                    <span class="template-card__icon" aria-hidden="true"><Icon name={entry.icon} size={20} /></span>
                    <span class="template-card__text">
                      <strong>
                        {i18nText(locale, entry.title)}
                        {entry.custom && (
                          <span class="rule-template-custom-badge">{t(locale, 'behavior.copy.tplCustomBadge')}</span>
                        )}
                      </strong>
                      <small>{i18nText(locale, entry.description)}</small>
                      {!available && requirements.missingTypeIds.length > 0 && (
                        <small class="template-card__missing">
                          {t(locale, 'behavior.copy.tplMissing', { name: requirements.missingTypeIds.join(', ') })}
                        </small>
                      )}
                      {!available && requirements.unknownTrigger && (
                        <small class="template-card__missing">
                          {t(locale, 'behavior.copy.tplUnknownTrigger', { name: entry.event.trigger })}
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
    };
  },
);

export default RuleTemplateModal;
</script>

<style scoped>
.rule-template-list-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.rule-template-list-row :deep(.ui-search) {
  flex: 1;
}

.rule-template-custom-badge {
  display: inline-block;
  margin-left: 8px;
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--tt-bg-soft, #23232e);
  color: var(--tt-text-dim, #a8a8b8);
  vertical-align: 1px;
}

.rule-template-dropzone {
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

.rule-template-dropzone:focus-visible {
  border-color: var(--tt-accent, #22c55e);
}

.rule-template-dropzone.is-dragging {
  border-color: var(--tt-accent, #22c55e);
  border-style: solid;
}

.rule-template-dropzone__title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
}

.rule-template-dropzone__hint {
  margin: 0;
  font-size: 12px;
  color: var(--tt-text-dim, #a8a8b8);
}

.rule-template-dropzone__actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.rule-template-staged {
  border: 1px solid var(--tt-border, #34343f);
  border-radius: 12px;
  padding: 12px;
  background: var(--tt-bg-soft, #1c1c26);
}

.rule-template-staged__head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.rule-template-staged__source {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 12px;
}

.rule-template-staged ul {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-template-staged li {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.rule-template-staged li small {
  margin-left: auto;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 11px;
}

.rule-template-success {
  color: var(--tt-accent, #22c55e);
  font-size: 13px;
}

.rule-template-hint {
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 13px;
}
</style>
