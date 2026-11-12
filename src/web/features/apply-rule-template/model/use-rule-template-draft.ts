import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import { isHttpUrl } from '@/entities/plugin/lib/connection-settings.ts';
import { applyRuleTemplate, ruleParamDefaults, type RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';

/** One candidate per unchanged draft, retained for a retry of an unconfirmed apply. */
export function useRuleTemplateDraft(
  working: () => boolean,
  submit: (actions: LiveAction[], event: LiveEvent, done: (success: boolean) => void) => void,
  appliedSuccessfully: () => void,
) {
  const { t } = useI18n();
  const selected = ref<RuleTemplate | null>(null);
  const eventName = ref('');
  const actionNames = ref<string[]>([]);
  const params = ref<JsonObject>({});
  const error = ref('');
  const applying = ref(false);
  const candidate = ref<ReturnType<typeof applyRuleTemplate> | null>(null);
  watch([eventName, actionNames, params], () => { if (!applying.value && !working()) candidate.value = null; }, { deep: true });

  function configure(template: RuleTemplate): void {
    candidate.value = null;
    selected.value = template;
    eventName.value = template.event.name;
    actionNames.value = template.actions.map((action) => action.name);
    params.value = ruleParamDefaults(template.params);
    error.value = '';
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
    if (!template || working() || applying.value) return;
    let applied: ReturnType<typeof applyRuleTemplate>;
    try {
      error.value = validate(template);
      if (error.value) return;
      applied = candidate.value ?? applyRuleTemplate(template, { ...params.value }, { eventName: eventName.value, actionNames: actionNames.value });
    }
    catch (failure) { error.value = t(failure instanceof ControlCallError && failure.code === 'too_large' ? 'libraryRecovery.expansionTooLarge' : 'libraryRecovery.expansionInvalid'); return; }
    candidate.value = applied; applying.value = true;
    submit(applied.actions, applied.event, (success) => { applying.value = false; if (success) { candidate.value = null; appliedSuccessfully(); } });
  }

  return { selected, eventName, actionNames, params, error, configure, apply };
}
