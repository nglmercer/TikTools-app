import { computed } from 'vue';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { useAutomationControls } from './use-automation-controls.ts';
import { createAutomationSyncAdapter, type AutomationSyncKind } from './automation-sync-adapter.ts';

/** Sync adapters for the Events and Actions tabs of the automation library. */
export function useAutomationSyncAdapters(): Record<AutomationSyncKind, SyncAdapter> {
  const controls = useAutomationControls();
  const events = computed(() => controls.snapshot.value.events);
  const actions = computed(() => controls.snapshot.value.actions);
  const common = {
    loaded: () => controls.persistent.value !== null,
    read: controls.readRecord,
    remove: controls.deleteRecord,
    t: (key: string) => i18n.global.t(key),
  };
  return {
    event: createAutomationSyncAdapter({
      ...common, kind: 'event', records: () => events.value,
      save: (record, base) => controls.saveEvent(record as LiveEvent, base),
    }),
    action: createAutomationSyncAdapter({
      ...common, kind: 'action', records: () => actions.value,
      save: (record, base) => controls.saveAction(record as LiveAction, base),
    }),
  };
}
