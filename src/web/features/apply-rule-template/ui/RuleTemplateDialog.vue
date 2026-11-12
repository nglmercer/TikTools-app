<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { BehaviorSnapshot, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { exportRuleTemplate, MAX_RULE_TEMPLATE_BYTES, type RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import type { RuleTemplateTab } from '../model/template-tab.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import CloudLibrary from '@/features/cloud-library/ui/CloudLibrary.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import BaseImportDialog from '@/shared/ui/import-dialog/BaseImportDialog.vue';
import type { ImportTab } from '@/shared/ui/import-dialog/import-types.ts';

import { useRuleTemplateDraft } from '../model/use-rule-template-draft.ts';
import { useRuleTemplateImport } from '../model/use-rule-template-import.ts';
import RuleTemplateConfiguration from './RuleTemplateConfiguration.vue';
import RuleTemplateCatalog from './RuleTemplateCatalog.vue';

type Screen = 'list' | 'configure';
/** Largest template file the manual import accepts; the parser refuses more text than this. */
const MAX_IMPORT_BYTES = MAX_RULE_TEMPLATE_BYTES * 4;

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
const tab = defineModel<RuleTemplateTab>('tab', { default: 'templates' });

const { t } = useI18n();
const id = useId();
const screen = ref<Screen>('list');
const query = ref('');

const draft = useRuleTemplateDraft(() => props.working, (actions, event, done) => emit('apply', actions, event, done), () => { open.value = false; });
const { selected, eventName, actionNames, params, error, apply } = draft;
const manualImport = useRuleTemplateImport((templates, done) => emit('import', templates, done));
const { staged, importErrors, imported, importing, stagedItems, importLabels, stage, onImportError, runImport } = manualImport;
const title = computed(() => {
  if (screen.value === 'configure' && selected.value) return localizedText(selected.value.title);
  return t('automations.templates.title');
});
const tabs = computed<ImportTab[]>(() => [
  { value: 'templates', label: t('importDialog.templatesTab'), icon: IconFlow },
  { value: 'cloud', label: t('importDialog.cloudTab'), icon: IconCloud },
  { value: 'manual', label: t('importDialog.manualTab'), icon: IconUpload },
]);
watch(open, (value) => {
  if (!value) return;
  screen.value = 'list';
  query.value = '';
  manualImport.reset();
});

function configure(template: RuleTemplate): void {
  draft.configure(template);
  screen.value = 'configure';
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
</script>

<template>
  <BaseImportDialog
    v-model:open="open"
    v-model:tab="tab"
    :title="title"
    :width="720"
    :tabs="tabs"
    :hide-tabs="screen === 'configure'"
    :max-bytes="MAX_IMPORT_BYTES"
    :staged="stagedItems"
    :staged-source="staged?.source"
    :busy="importing"
    :errors="tab === 'manual' ? importErrors : []"
    :success="tab === 'manual' && imported > 0 ? t('automations.import.done', { count: imported }) : ''"
    :labels="importLabels"
    @content="stage"
    @error="onImportError"
    @clear="staged = null"
    @import="runImport"
  >
    <template #templates>
      <RuleTemplateConfiguration
        v-if="screen === 'configure' && selected"
        v-model:event-name="eventName"
        v-model:action-names="actionNames"
        v-model:params="params"
        :selected="selected"
        :field-id="id"
        :error="error"
      />
      <RuleTemplateCatalog
        v-else
        v-model:query="query"
        :snapshot="snapshot"
        :custom-templates="customTemplates"
        :custom-error="customError"
        @configure="configure"
      />
    </template>

    <template #cloud>
      <CloudLibrary bare />
    </template>

    <template #manual-lead>
      <p class="rule-templates__lead">
        {{ t('automations.templates.importLead') }}
      </p>
    </template>

    <template
      v-if="screen === 'configure' && selected"
      #footer
    >
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
  </BaseImportDialog>
</template>

<style src="./rule-template-dialog.css"></style>
