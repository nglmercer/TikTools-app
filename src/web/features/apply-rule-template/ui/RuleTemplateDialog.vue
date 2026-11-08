<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { BEHAVIOR_TRIGGERS } from '../../../../automation/behavior/schema.ts';
import type { BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import { availableActionTypeIds, matchesQuery } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import { isHttpUrl } from '@/entities/plugin/lib/connection-settings.ts';
import {
  applyRuleTemplate,
  BUILTIN_RULE_TEMPLATES,
  exportRuleTemplate,
  missingRuleTemplateRequirements,
  ruleParamDefaults,
  type RuleTemplate,
} from '@/entities/rule-template/lib/rule-templates.ts';
import { parseRuleTemplateImport } from '@/entities/rule-template/model/rule-templates-state.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

type Screen = 'list' | 'configure' | 'import';

const props = withDefaults(
  defineProps<{
    snapshot: BehaviorSnapshot;
    customTemplates: readonly RuleTemplate[];
    customError?: string | null;
    working?: boolean;
  }>(),
  { customError: null, working: false },
);

const emit = defineEmits<{
  apply: [actions: LiveAction[], event: LiveEvent, done: (success: boolean) => void];
  import: [templates: RuleTemplate[], done: (added: number) => void];
  deleteCustom: [id: string];
}>();

const open = defineModel<boolean>('open', { default: false });

const { t } = useI18n();
const { triggerName } = useAutomationText();
const id = useId();
const screen = ref<Screen>('list');
const query = ref('');
const selected = ref<RuleTemplate | null>(null);
const eventName = ref('');
const actionNames = ref<string[]>([]);
const params = ref<JsonObject>({});
const error = ref('');
const staged = ref<{ source: string; templates: RuleTemplate[] } | null>(null);
const importErrors = ref<string[]>([]);
const imported = ref(0);
const importing = ref(false);
const applying = ref(false);
const candidate = ref<ReturnType<typeof applyRuleTemplate> | null>(null);
watch([eventName, actionNames, params], () => { if (!applying.value && !props.working) candidate.value = null; }, { deep: true });

const templates = computed(() => [...BUILTIN_RULE_TEMPLATES, ...props.customTemplates]);
const availableTypes = computed(() => availableActionTypeIds(props.snapshot.plugins, props.snapshot.actionTypes));
const knownTriggers = computed(() => [...BEHAVIOR_TRIGGERS, ...(props.snapshot.eventTypes ?? []).map((entry) => entry.type)]);
const entries = computed(() =>
  templates.value
    .filter((template) =>
      matchesQuery(query.value, localizedText(template.title), localizedText(template.description), template.event.trigger),
    )
    .map((template) => {
      const requirements = missingRuleTemplateRequirements(template, availableTypes.value, knownTriggers.value);
      const problems = [
        ...(requirements.missingTypeIds.length ? [t('automations.templates.requires', { name: requirements.missingTypeIds.join(', ') })] : []),
        ...(requirements.unknownTrigger ? [t('automations.templates.unknownTrigger', { name: template.event.trigger })] : []),
      ];
      return { template, problems };
    }),
);
const title = computed(() => {
  if (screen.value === 'import') return t('automations.templates.importTitle');
  if (screen.value === 'configure' && selected.value) return localizedText(selected.value.title);
  return t('automations.templates.title');
});

watch(open, (value) => {
  if (!value) return;
  screen.value = 'list';
  query.value = '';
  staged.value = null;
  importErrors.value = [];
  imported.value = 0;
});

function configure(template: RuleTemplate): void {
  candidate.value = null;
  selected.value = template;
  eventName.value = template.event.name;
  actionNames.value = template.actions.map((action) => action.name);
  params.value = ruleParamDefaults(template.params);
  error.value = '';
  screen.value = 'configure';
}

function requiredParams(template: RuleTemplate): string[] {
  const required = template.params?.required;
  return Array.isArray(required) ? required.filter((entry): entry is string => typeof entry === 'string') : [];
}

function validate(template: RuleTemplate): string {
  if (!eventName.value.trim() || actionNames.value.some((name) => !name.trim())) return t('automations.templates.required');
  for (const key of requiredParams(template)) {
    const value = params.value[key];
    if (value === undefined || value === null || (typeof value === 'string' && value.trim() === '')) {
      return t('automations.templates.required');
    }
  }
  if ((template.id === 'gift-webhook' || template.id === 'chat-webhook') && !isHttpUrl(String(params.value['url'] ?? ''))) {
    return t('automations.templates.invalidUrl');
  }
  return '';
}

function apply(): void {
  const template = selected.value;
  if (!template || props.working || applying.value) return;
  let applied: ReturnType<typeof applyRuleTemplate>;
  try {
    error.value = validate(template);
    if (error.value) return;
    applied = candidate.value ?? applyRuleTemplate(template, { ...params.value }, { eventName: eventName.value, actionNames: actionNames.value });
  }
  catch (failure) { error.value = t(failure instanceof ControlCallError && failure.code === 'too_large' ? 'libraryRecovery.expansionTooLarge' : 'libraryRecovery.expansionInvalid'); return; }
  candidate.value = applied; applying.value = true;
  emit('apply', applied.actions, applied.event, (success) => { applying.value = false; if (success) { candidate.value = null; open.value = false; } });
}

function download(template: RuleTemplate): void {
  const url = URL.createObjectURL(new Blob([exportRuleTemplate(template)], { type: 'application/json' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${template.id}.tiktemplate.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

function remove(template: RuleTemplate): void {
  emit('deleteCustom', template.id);
  screen.value = 'list';
}

function stage(text: string, source: string): void {
  imported.value = 0;
  const result = parseRuleTemplateImport(text);
  staged.value = result.templates.length ? { source, templates: result.templates } : null;
  importErrors.value = result.templates.length || result.errors.length ? result.errors : [t('automations.import.empty')];
}

function onImportError(message: string): void {
  staged.value = null;
  importErrors.value = [message];
}

function runImport(): void {
  const pending = staged.value;
  if (!pending || importing.value) return;
  importing.value = true;
  emit('import', pending.templates, (added) => {
    importing.value = false;
    imported.value = added;
    if (added > 0) staged.value = null;
  });
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="title"
    :width="720"
  >
    <template v-if="screen === 'list'">
      <p class="rule-templates__lead">
        {{ t('automations.templates.lead') }}
      </p>
      <div class="rule-templates__toolbar">
        <BaseTextField
          v-model="query"
          class="rule-templates__search"
          type="search"
          :placeholder="t('automations.templates.search')"
          :aria-label="t('automations.templates.search')"
        >
          <template #prefix>
            <IconSearch />
          </template>
        </BaseTextField>
        <BaseButton @click="screen = 'import'">
          {{ t('automations.templates.import') }}
        </BaseButton>
      </div>
      <p
        v-if="customError"
        class="rule-templates__error"
        role="alert"
      >
        {{ customError }}
      </p>
      <p
        v-if="!entries.length"
        class="rule-templates__muted"
      >
        {{ t('automations.templates.empty') }}
      </p>
      <div class="rule-templates__grid">
        <button
          v-for="entry in entries"
          :key="`${entry.template.custom ? 'custom' : 'builtin'}-${entry.template.id}`"
          class="rule-templates__card"
          type="button"
          :disabled="entry.problems.length > 0"
          @click="configure(entry.template)"
        >
          <span class="rule-templates__card-title">
            {{ localizedText(entry.template.title) }}
            <span
              v-if="entry.template.custom"
              class="rule-templates__badge"
            >{{ t('automations.templates.imported') }}</span>
          </span>
          <span class="rule-templates__card-description">{{ localizedText(entry.template.description) }}</span>
          <span class="rule-templates__card-trigger">{{ triggerName(entry.template.event.trigger, snapshot.eventTypes ?? []) }}</span>
          <span
            v-for="problem in entry.problems"
            :key="problem"
            class="rule-templates__problem"
          >{{ problem }}</span>
        </button>
      </div>
    </template>

    <template v-else-if="screen === 'configure' && selected">
      <p class="rule-templates__lead">
        {{ localizedText(selected.description) }}
      </p>
      <BaseFormGrid>
        <BaseFormRow
          :for="`${id}-event`"
          :label="t('automations.templates.eventName')"
        >
          <BaseTextField
            :id="`${id}-event`"
            v-model="eventName"
          />
        </BaseFormRow>
        <BaseFormRow
          v-for="(action, index) in selected.actions"
          :key="`${action.typeId}-${index}`"
          :for="`${id}-action-${index}`"
          :label="selected.actions.length > 1 ? t('automations.templates.actionNameOf', { name: action.typeId }) : t('automations.templates.actionName')"
        >
          <BaseTextField
            :id="`${id}-action-${index}`"
            v-model="actionNames[index]"
          />
        </BaseFormRow>
      </BaseFormGrid>
      <BaseGroupBox
        v-if="selected.params"
        class="rule-templates__params"
        :title="t('automations.templates.options')"
      >
        <BaseSchemaForm
          v-model="params"
          :schema="selected.params"
        />
      </BaseGroupBox>
      <p
        v-if="error"
        class="rule-templates__error"
        role="alert"
      >
        {{ error }}
      </p>
      <p class="rule-templates__muted">
        {{ t('automations.templates.reviewHint') }}
      </p>
    </template>

    <template v-else>
      <p class="rule-templates__lead">
        {{ t('automations.templates.importLead') }}
      </p>
      <BaseFileDrop
        :title="t('automations.import.dropTitle')"
        :hint="t('automations.import.dropHint')"
        :browse-label="t('automations.import.browse')"
        :paste-label="t('automations.import.paste')"
        :paste-blocked-label="t('automations.import.pasteBlocked')"
        :clipboard-label="t('automations.import.clipboard')"
        :unreadable-label="t('automations.import.unreadable')"
        :disabled="importing"
        @content="stage"
        @error="onImportError"
      />
      <div
        v-if="staged"
        class="rule-templates__staged"
      >
        <p>
          <strong>{{ t('automations.import.staged', { count: staged.templates.length }) }}</strong>
          {{ t('automations.import.from', { source: staged.source }) }}
        </p>
        <ul>
          <li
            v-for="template in staged.templates"
            :key="template.id"
          >
            {{ localizedText(template.title) }}
          </li>
        </ul>
        <BaseButton
          size="sm"
          @click="staged = null"
        >
          {{ t('automations.import.clear') }}
        </BaseButton>
      </div>
      <p
        v-if="imported > 0"
        class="rule-templates__success"
        role="status"
      >
        {{ t('automations.import.done', { count: imported }) }}
      </p>
      <p
        v-for="message in importErrors"
        :key="message"
        class="rule-templates__error"
        role="alert"
      >
        {{ message }}
      </p>
    </template>

    <template #footer>
      <template v-if="screen === 'configure' && selected">
        <BaseButton
          variant="primary"
          @click="apply"
        >
          {{ t('automations.templates.create') }}
        </BaseButton>
        <BaseButton
          v-if="selected.custom"
          @click="download(selected)"
        >
          {{ t('automations.templates.export') }}
        </BaseButton>
        <BaseButton
          v-if="selected.custom"
          variant="danger"
          @click="remove(selected)"
        >
          {{ t('automations.templates.delete') }}
        </BaseButton>
        <BaseButton @click="screen = 'list'">
          {{ t('automations.templates.back') }}
        </BaseButton>
      </template>
      <template v-else-if="screen === 'import'">
        <BaseButton
          variant="primary"
          :disabled="!staged || importing"
          @click="runImport"
        >
          {{ t('automations.import.run') }}
        </BaseButton>
        <BaseButton @click="screen = 'list'">
          {{ t('automations.templates.back') }}
        </BaseButton>
      </template>
      <BaseButton
        v-else
        @click="open = false"
      >
        {{ t('common.close') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.rule-templates__lead {
  margin: 0 0 var(--space-4);
  color: var(--color-text-muted);
}

.rule-templates__toolbar {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
}

.rule-templates__search {
  flex: 1;
}

.rule-templates__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: var(--space-2);
  max-height: min(55vh, 460px);
  overflow-y: auto;
}

.rule-templates__card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-start;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  text-align: left;
  cursor: pointer;
}

.rule-templates__card:hover:not(:disabled) {
  border-color: var(--color-border-strong);
  background: var(--color-bg-control-hover);
}

.rule-templates__card:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.rule-templates__card:disabled {
  opacity: 0.55;
  cursor: default;
}

.rule-templates__card-title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-weight: var(--font-weight-semibold);
}

.rule-templates__card-description {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.rule-templates__card-trigger {
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
}

.rule-templates__badge {
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-selected);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-regular);
}

.rule-templates__problem {
  color: var(--color-warning);
  font-size: var(--font-size-xs);
}

.rule-templates__params {
  margin-top: var(--space-5);
}

.rule-templates__staged {
  margin-top: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.rule-templates__staged p,
.rule-templates__staged ul {
  margin: 0 0 var(--space-3);
}

.rule-templates__muted {
  margin: var(--space-4) 0 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.rule-templates__error,
.rule-templates__success {
  margin: var(--space-3) 0 0;
  color: var(--color-danger-hover);
}

.rule-templates__success {
  color: var(--color-success);
}
</style>
