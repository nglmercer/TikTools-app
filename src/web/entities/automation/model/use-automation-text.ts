import { useI18n } from 'vue-i18n';
import type { EventFilter, FilterOperator, LiveAction, LiveEvent, PluginEventType } from '../../../../automation/behavior/types.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { actionNames, triggerLabel, triggerMessageKey, triggerOptions } from '../lib/automation-records.ts';
import { conditionFields, describeFilter } from '../lib/conditions.ts';

export function useAutomationText() {
  const { t, locale } = useI18n();

  const builtinTrigger = (trigger: string): string => t(triggerMessageKey(trigger));
  const operatorName = (operator: FilterOperator): string => t(`automations.operators.${operator}`);
  const triggerName = (trigger: string, eventTypes: readonly PluginEventType[]): string =>
    triggerLabel(trigger, eventTypes, builtinTrigger, localizedText);
  const triggers = (eventTypes: readonly PluginEventType[]) => triggerOptions(eventTypes, builtinTrigger, localizedText);

  function filterText(filter: EventFilter, trigger: string): string {
    const field = conditionFields(trigger, locale.value).find((entry) => entry.path === filter.path);
    return describeFilter(filter, field?.label ?? '', operatorName(filter.operator));
  }

  function joinList(items: readonly string[], conjunction = t('automations.event.and')): string {
    if (items.length === 0) return t('automations.event.nothing');
    if (items.length === 1) return items[0]!;
    return `${items.slice(0, -1).join(', ')} ${conjunction} ${items.at(-1)!}`;
  }

  function sentence(event: LiveEvent, actions: readonly LiveAction[], eventTypes: readonly PluginEventType[]): string {
    const filters = event.filters.map((filter) => filterText(filter, event.trigger));
    const names = actionNames(event, actions);
    const conjunction = event.matchMode === 'or' ? t('automations.event.or') : t('automations.event.and');
    return t('automations.event.sentence', {
      trigger: triggerName(event.trigger, eventTypes).toLocaleLowerCase(locale.value),
      condition: filters.length ? t('automations.event.sentenceCondition', { filters: joinList(filters, conjunction) }) : '',
      actions: event.runMode === 'random' && names.length > 1 ? t('automations.event.sentenceOneOf', { names: joinList(names) }) : joinList(names),
    });
  }

  return { t, locale, operatorName, triggerName, triggers, filterText, sentence };
}
