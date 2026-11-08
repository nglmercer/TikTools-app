
import { ref } from 'vue';

import type { ControlClient } from '@/shared/api/control-client.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import {
  BUILTIN_RULE_TEMPLATES,
  MAX_RULE_TEMPLATE_BYTES,
  parseRuleTemplateList,
  type RuleTemplate,
} from '@/entities/rule-template/lib/rule-templates.ts';

const CUSTOM_TEMPLATES_KEY = 'behavior.templates.custom';

function freshCustomId(): string {
  return `custom-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function parseRuleTemplateImport(text: string): { templates: RuleTemplate[]; errors: string[] } {
  if (text.length > MAX_RULE_TEMPLATE_BYTES * 4) {
    return { templates: [], errors: ['import is too large (max 256 KB of text)'] };
  }
  let value: unknown;
  try {
    value = JSON.parse(text) as unknown;
  } catch {
    return { templates: [], errors: ['import is not valid JSON'] };
  }
  return parseRuleTemplateList(value);
}

export function useRuleTemplates(control: ControlClient, persistence: () => boolean | null = () => null) {
  const custom = ref<RuleTemplate[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const builtinIds = new Set(BUILTIN_RULE_TEMPLATES.map((template) => template.id));

  const loadCustom = async (): Promise<void> => {
    loading.value = true;
    error.value = null;
    try {
      const result = await control.call<{ state: Record<string, string> }>('app.state.get', {
        keys: [CUSTOM_TEMPLATES_KEY],
      });
      const raw = result.state[CUSTOM_TEMPLATES_KEY];
      if (!raw || raw.trim() === '') {
        custom.value = [];
        return;
      }
      const { templates, errors } = parseRuleTemplateImport(raw);
      custom.value = templates.map((template) => ({ ...template, custom: true }));
      if (errors.length > 0) error.value = errors.join('; ');
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  };

  const persist = async (next: RuleTemplate[]): Promise<void> => {
    if (persistence() !== false) throw new ControlCallError('capability_unavailable', 'Persistent template changes require the native atomic library.');
    await control.call('app.state.set', {
      key: CUSTOM_TEMPLATES_KEY,
      value: JSON.stringify(next.map(({ custom: _dropped, ...document }) => {
        void _dropped;
        return document;
      })),
    });
    custom.value = next;
  };

  const importTemplates = async (templates: RuleTemplate[]): Promise<number> => {
    const taken = new Set([...builtinIds, ...custom.value.map((template) => template.id)]);
    const next = [...custom.value];
    for (const template of templates) {
      const id = taken.has(template.id) ? freshCustomId() : template.id;
      taken.add(id);
      next.push({ ...template, id, custom: true });
    }
    error.value = null;
    try {
      await persist(next);
    } catch (failure) {
      error.value = errorMessage(failure);
      return 0;
    }
    return templates.length;
  };

  const deleteCustom = async (id: string): Promise<void> => {
    error.value = null;
    try {
      await persist(custom.value.filter((template) => template.id !== id));
    } catch (failure) {
      error.value = errorMessage(failure);
    }
  };

  return { custom, loading, error, loadCustom, importTemplates, deleteCustom };
}
