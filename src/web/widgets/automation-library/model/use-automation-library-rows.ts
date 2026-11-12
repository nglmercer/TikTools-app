import { computed, type Ref } from 'vue';
import { actionNames, actionTypeOf, describeAction, lastRunByAction, matchesQuery, sortRecords, type RecordSort } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import { formatClockTime } from '@/entities/analytics/lib/analytics-range.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import type { TableColumn } from '@/shared/ui/table/table-types.ts';
import type { AutomationLibraryProps } from './library-props.ts';

/** Read-only rows/columns. Query, ordering and selection remain owned by the caller. */
export function useAutomationLibraryRows(props: Readonly<AutomationLibraryProps>, filters: {
  eventQuery: Readonly<Ref<string>>;
  actionQuery: Readonly<Ref<string>>;
  eventSort: Readonly<Ref<RecordSort>>;
  actionSort: Readonly<Ref<RecordSort>>;
}) {
  const { eventQuery, actionQuery, eventSort, actionSort } = filters;
  const { t, locale, triggerName } = useAutomationText();
  // Phones drop the least essential column so names and triggers stay readable.
  const isPhone = useMediaQuery('(width < 600px)');
  const eventColumns = computed<TableColumn[]>(() => [
    { key: 'enabled', label: t('automations.library.columns.enabled'), width: '52px', align: 'center' },
    { key: 'name', label: t('automations.library.columns.name'), width: isPhone.value ? '50%' : '32%' },
    { key: 'trigger', label: t('automations.library.columns.trigger'), width: isPhone.value ? undefined : '30%' },
    ...(isPhone.value ? [] : [{ key: 'actions', label: t('automations.library.columns.actions') }]),
  ]);
  const actionColumns = computed<TableColumn[]>(() => [
    { key: 'enabled', label: t('automations.library.columns.enabled'), width: '52px', align: 'center' },
    { key: 'name', label: t('automations.library.columns.name'), width: isPhone.value ? '55%' : '38%' },
    { key: 'type', label: t('automations.library.columns.type') },
    ...(isPhone.value ? [] : [{ key: 'lastRun', label: t('automations.library.columns.lastRun'), width: '110px', align: 'end' as const }]),
  ]);

  const eventRows = computed(() =>
    sortRecords(props.events, eventSort.value)
      .map((event) => ({
        id: event.id,
        name: event.name,
        enabled: event.enabled,
        trigger: triggerName(event.trigger, props.eventTypes),
        conditions: t('automations.library.conditionCount', event.filters.length),
        actions: actionNames(event, props.actions),
      }))
      .filter((row) => matchesQuery(eventQuery.value, row.name, row.trigger, ...row.actions)),
  );

  const lastRuns = computed(() => lastRunByAction(props.runs));

  const actionRows = computed(() =>
    sortRecords(props.actions, actionSort.value)
      .map((action) => {
        const type = actionTypeOf(action, props.actionTypes);
        const run = lastRuns.value.get(action.id);
        return {
          id: action.id,
          name: action.name,
          enabled: action.enabled,
          typeId: action.typeId,
          type: type ? localizedText(type.title) : action.typeId,
          detail: describeAction(action),
          missing: !props.availableTypes.has(action.typeId),
          lastRun: run ? formatClockTime(run.at / 1000, locale.value) : t('automations.library.never'),
          failed: run?.status === 'error',
        };
      })
      .filter((row) => matchesQuery(actionQuery.value, row.name, row.type, row.detail)),
  );

  return { eventColumns, actionColumns, eventRows, actionRows };
}
export type AutomationEventRow = ReturnType<typeof useAutomationLibraryRows>['eventRows']['value'][number];
export type AutomationActionRow = ReturnType<typeof useAutomationLibraryRows>['actionRows']['value'][number];
