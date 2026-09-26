<script lang="tsx">
import { computed, ref } from 'vue';
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
import { ImportDropzone } from './ImportDropzone.vue';
import { StagedTemplateList } from './StagedTemplateList.vue';

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
      const { templates, errors } = parseRuleTemplateImport(text);
      if (templates.length === 0 && errors.length === 0) {
        staged.value = null;
        importErrors.value = [t(props.locale, 'behavior.copy.tplImportEmpty')];
        return;
      }
      staged.value = templates.length > 0 ? { source, templates } : null;
      importErrors.value = errors;
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
      } catch {
        // Template import failures stay silent, matching the previous behavior.
      } finally {
        importing.value = false;
      }
    };

    return () => {
      const locale = props.locale;
      const current = screen.value;

      if (current.kind === 'import') {
        const pending = staged.value;
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
              <ImportDropzone
                locale={locale}
                title={t(locale, 'behavior.copy.tplDropTitle')}
                hint={t(locale, 'behavior.copy.tplDropHint')}
                browseLabel={t(locale, 'behavior.copy.tplBrowse')}
                pasteLabel={t(locale, 'behavior.copy.tplPaste')}
                clipboardSource={t(locale, 'behavior.copy.tplClipboardSource')}
                pastePressKeys={t(locale, 'behavior.copy.tplPastePressKeys')}
                unreadableMessage={t(locale, 'behavior.copy.tplImportUnreadable')}
                onContent={(text, source) => stageContent(text, source)}
                onReadError={(message) => {
                  staged.value = null;
                  importErrors.value = [message];
                }}
              />
              {pending && (
                <StagedTemplateList
                  locale={locale}
                  headline={t(locale, 'behavior.copy.tplStaged', { count: pending.templates.length })}
                  templates={pending.templates}
                  source={pending.source}
                  stagedFrom={t(locale, 'behavior.copy.tplStagedFrom', { source: pending.source })}
                  clearLabel={t(locale, 'behavior.copy.tplClearStaged')}
                  onClear={() => { staged.value = null; }}
                />
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

.rule-template-success {
  color: var(--tt-accent, #22c55e);
  font-size: 13px;
}
</style>
