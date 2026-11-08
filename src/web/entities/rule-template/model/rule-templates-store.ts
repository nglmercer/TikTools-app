import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useRuleTemplates } from './rule-templates-state.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { libraryJournal } from './library-journal.ts';
import { BUILTIN_RULE_TEMPLATES, parseRuleTemplate, type RuleTemplate } from '../lib/rule-templates.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';

export const useRuleTemplatesStore = defineStore('rule-templates', () => {
  const client = useControlClient(); const automation = useAutomationStore(); const legacy = useRuleTemplates(client, () => automation.persistent); const library = libraryJournal(client);
  const error = ref<string | null>(null);
  const stagedImports = new WeakMap<RuleTemplate[], RuleTemplate[]>();
  let customSerial = 0;
  const memory = computed(() => automation.persistent === false);
  // Native catalogs preserve raw documents. Only this UI projection applies
  // grammar defaults; the coherent catalog and its reviewed fence stay intact.
  const custom = computed(() => memory.value ? legacy.custom.value : (library.snapshot.value?.catalog.templates ?? []).map((template) => {
    const parsed = parseRuleTemplate(template);
    if (!parsed.ok) throw new ControlCallError('protocol', 'The saved template cannot be displayed safely.');
    return { ...parsed.template, custom: true };
  }));
  async function importTemplates(templates: RuleTemplate[]): Promise<number> {
    if (memory.value) return legacy.importTemplates(templates);
    error.value = null;
    const taken = new Set([...BUILTIN_RULE_TEMPLATES, ...custom.value].map((template) => template.id));
    const captured = stagedImports.get(templates) ?? templates.map(({ custom: _custom, ...template }) => {
      void _custom;
      let id = template.id;
      while (taken.has(id)) id = `custom-${Date.now().toString(36)}-${(++customSerial).toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
      taken.add(id); return { ...template, id };
    });
    stagedImports.set(templates, captured);
    try { await library.request({ operation: 'import-templates', templates: captured }, library.snapshot.value); return captured.length; }
    catch (failure) { if (!(failure instanceof ControlCallError && failure.code === 'cancelled')) error.value = errorMessage(failure); return 0; }
  }
  async function deleteCustom(id: string): Promise<void> {
    if (memory.value) return legacy.deleteCustom(id);
    error.value = null;
    try { await library.request({ operation: 'delete-template', id }, library.snapshot.value); }
    catch (failure) { if (!(failure instanceof ControlCallError && failure.code === 'cancelled')) error.value = errorMessage(failure); }
  }
  return { custom, error, loading: computed(() => memory.value ? legacy.loading.value : library.busy.value),
    loadCustom: async () => { if (memory.value) await legacy.loadCustom(); else { try { await library.refreshSnapshot(); } catch (failure) { error.value = errorMessage(failure); } } }, importTemplates, deleteCustom };
});
