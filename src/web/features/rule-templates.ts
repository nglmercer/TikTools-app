/**
 * Rule template store: built-ins ship in code, user templates persist in
 * host app.state (`behavior.templates.custom`) so imports survive restarts.
 * Import accepts one document, an array, or `{templates: [...]}` from a file
 * or pasted text; export downloads a portable JSON document per template.
 */

import { ref } from 'vue';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import {
  BUILTIN_RULE_TEMPLATES,
  MAX_RULE_TEMPLATE_BYTES,
  parseRuleTemplateList,
  type RuleTemplate,
} from '../views/behavior/rule-templates.ts';

const CUSTOM_TEMPLATES_KEY = 'behavior.templates.custom';

function freshCustomId(): string {
  return `custom-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/** Rejects oversized payloads before parsing. */
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

export function useRuleTemplates(control: ControlClient) {
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
    await control.call('app.state.set', {
      key: CUSTOM_TEMPLATES_KEY,
      value: JSON.stringify(next.map(({ custom: _dropped, ...document }) => {
        void _dropped;
        return document;
      })),
    });
    custom.value = next;
  };

  /** Imports parsed templates, re-keying ids that collide with built-ins or stored customs. */
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
