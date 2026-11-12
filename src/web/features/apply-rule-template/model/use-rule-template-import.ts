import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { parseRuleTemplateImport } from '@/entities/rule-template/model/rule-templates-state.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import type { StagedImportItem } from '@/shared/ui/import-dialog/import-types.ts';

export function useRuleTemplateImport(submit: (templates: RuleTemplate[], done: (added: number) => void) => void) {
  const { t } = useI18n();
  const staged = ref<{ source: string; templates: RuleTemplate[] } | null>(null);
  const importErrors = ref<string[]>([]);
  const imported = ref(0);
  const importing = ref(false);
  const stagedItems = computed<StagedImportItem[]>(() =>
    (staged.value?.templates ?? []).map((template) => ({ key: template.id, label: localizedText(template.title) })),
  );
  const importLabels = computed(() => ({
    dropTitle: t('automations.import.dropTitle'),
    dropHint: t('automations.import.dropHint'),
    browse: t('automations.import.browse'),
    paste: t('automations.import.paste'),
    pasteBlocked: t('automations.import.pasteBlocked'),
    clipboard: t('automations.import.clipboard'),
    unreadable: t('automations.import.unreadable'),
    staged: t('automations.import.staged', { count: stagedItems.value.length }),
    clear: t('automations.import.clear'),
    run: t('automations.import.run'),
  }));

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
    submit(pending.templates, (added) => {
      importing.value = false;
      imported.value = added;
      if (added > 0) staged.value = null;
    });
  }
  function reset(): void {
    staged.value = null;
    importErrors.value = [];
    imported.value = 0;
  }
  return { staged, importErrors, imported, importing, stagedItems, importLabels, stage, onImportError, runImport, reset };
}
