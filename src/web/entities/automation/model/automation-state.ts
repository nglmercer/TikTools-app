import { createAutomationProjection } from './automation-projection.ts';
import { createAutomationRunBuffer, type AutomationRunsResult } from './automation-run-buffer.ts';
import { createAutomationRecordGateway } from './automation-record-gateway.ts';
import { createAutomationExecution } from './automation-execution.ts';
export type { AutomationRunsResult } from './automation-run-buffer.ts';
export type { AutomationFireResult } from './automation-execution.ts';
import { ownControlSubscriptions } from '@/shared/api/control-owner.ts';
import { ref } from 'vue';

import type {
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
} from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { callWithMeta, ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { assertRecord, automationVersions,
  type AutomationHost, type AutomationRevision, type AutomationWriteBase,
} from './record-versions.ts';
import { definitionJournal } from './definition-journal.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';

export interface LastHotkeyEvent {
  key: string;
  modifiers: string;
  backend: string;
  at: number;
}

export function useAutomation(control: ControlClient) {
  // Keep native journals shared with all review surfaces on the injected host.
  const definitions = definitionJournal(control);
  const library = libraryJournal(control);
  const owner = ownControlSubscriptions(control);
  control = owner.client;
  let disposed = false;

  const projection = createAutomationProjection(library);
  const { behavior, behaviorVersions, persistent, sourceHost, revisionFor, pluginPages, pluginUis } = projection;
  const runBuffer = createAutomationRunBuffer(control, () => disposed);
  const { behaviorRuns, applyRuns, setPresentationActive } = runBuffer;
  const behaviorError = ref('');
  const hotkeyStatus = ref<HotkeyStatusData | null>(null);
  const lastHotkeyEvent = ref<LastHotkeyEvent | null>(null);
  const hotkeyAccessPending = ref(false);

  const clearBehaviorError = (): void => {
    behaviorError.value = '';
  };

  const applySnapshot = (snapshot: BehaviorSnapshot, versions: ReadonlyMap<string, string>, persistence: boolean | null, host: AutomationHost | null): void => {
    if (disposed) return;
    projection.applySnapshot(snapshot, versions, persistence, host);
    if (
      !snapshot.plugins.some(
        (plugin) => plugin.descriptor.id === 'hotkeys' && plugin.installed && plugin.enabled,
      )
    ) {
      hotkeyStatus.value = null;
    }
    behaviorError.value = '';
  };

  control.onPush('hotkey-status', (message) => {
    if (message.type !== 'hotkey-status') return;
    hotkeyStatus.value = message.status;
  });
  control.onTopic<{ pluginId?: unknown; eventType?: unknown; event?: unknown }>(
    'plugin.event',
    (data) => {
      if (!data || data.eventType !== 'hotkey.pressed') return;
      const event = data.event as { data?: { key?: unknown; modifiers?: unknown; backend?: unknown } } | undefined;
      const detail = event?.data;
      if (!detail || typeof detail.key !== 'string') return;
      lastHotkeyEvent.value = {
        key: detail.key,
        modifiers: typeof detail.modifiers === 'string' ? detail.modifiers : '',
        backend: typeof detail.backend === 'string' ? detail.backend : '',
        at: Date.now(),
      };
    },
  );
  control.onTopic<{ status: HotkeyStatusData }>('plugin.status', (data) => {
    if (!data || !data.status || !Array.isArray(data.status.backends)) return;
    hotkeyStatus.value = data.status;
  });
  control.onPush('behavior-error', (message) => {
    if (message.type !== 'behavior-error') return;
    behaviorError.value = message.message;
  });
  control.onPush('automation-error', (message) => {
    if (message.type !== 'automation-error') return;
    behaviorError.value = message.message;
  });
  let refreshRevision = 0;
  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  const scheduleRefresh = (): void => {
    if (disposed) return;
    if (refreshTimer !== null) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      refreshTimer = null;
      void refresh().catch((failure: unknown) => {
        if (disposed) return;
        console.warn(`automation refresh failed: ${errorMessage(failure)}`);
      });
    }, 200);
  };
  control.onTopic('workflow.changed', () => {
    scheduleRefresh();
  });

  const refresh = async (): Promise<void> => {
    if (disposed) return;
    const revision = ++refreshRevision;
    const runsAtStart = runBuffer.revision();
    try {
      const [runs, info] = await Promise.all([
        control.call<AutomationRunsResult>('automation.runs', {}),
        control.call<{ features?: { persistence?: boolean } }>('system.info', {}).catch(() => null),
      ]);
      const memory = info?.features?.persistence === false;
      const coherent = memory ? null : await library.readSnapshot();
      const snapshot = coherent ? { result: coherent.definitions, meta: { resourceVersions: coherent.resourceVersions } } : await callWithMeta<BehaviorSnapshot>(control, 'automation.snapshot', {});
      if (disposed || revision !== refreshRevision) return;
      const versions = automationVersions(snapshot.meta);
      if (!snapshot.result || !Array.isArray(snapshot.result.actions) || !Array.isArray(snapshot.result.events) ||
          !Array.isArray(snapshot.result.plugins) || !Array.isArray(snapshot.result.actionTypes) ||
          !snapshot.result.plugins.every((p) => p?.descriptor && typeof p.descriptor.id === 'string')) {
        throw new ControlCallError('protocol', 'Invalid automation snapshot.');
      }
      for (const [kind, rows] of [['action', snapshot.result.actions], ['event', snapshot.result.events]] as const) {
        const seen = new Set<string>();
        for (const row of rows) {
          assertRecord(row, kind, row?.id);
          if (typeof row.id !== 'string' || !row.id || seen.has(row.id)) throw new ControlCallError('protocol', 'Ambiguous automation snapshot.');
          seen.add(row.id);
        }
      }
      const host = coherent && { hostId: coherent.hostId, hostInstanceId: coherent.hostInstanceId };
      if (coherent) { library.installSnapshot(coherent); if (library.snapshot.value !== coherent) return; }
      applySnapshot(snapshot.result, versions, typeof info?.features?.persistence === 'boolean' ? info.features.persistence : null, host);
      if (runsAtStart === runBuffer.revision()) applyRuns(runs.runs);
    } catch (failure) {
      if (disposed || revision !== refreshRevision) return;
      behaviorError.value = errorMessage(failure);
    }
  };

  const mutate = async (work: () => Promise<unknown>): Promise<void> => {
    clearBehaviorError();
    try {
      await work();
    } catch (failure) {
      if (disposed) return;
      behaviorError.value = errorMessage(failure);
      return;
    }
    await refresh();
  };

  const records = createAutomationRecordGateway(control, {
    persistent,
    definitions,
    isDisposed: () => disposed,
    invalidateRefresh: () => { ++refreshRevision; },
    refresh,
  });
  const { checkedOptions, readRecord, deleteRecord, setRecordEnabled, saveRecordAndRefresh } = records;
  const { behaviorTestRuns, testRecord, handleFireEvent, handleAnalyzeScript } = createAutomationExecution(control, {
    isDisposed: () => disposed,
    error: behaviorError,
    clearError: clearBehaviorError,
  });

  const handleSaveAction = (action: LiveAction, base: AutomationWriteBase): void => {
    void mutate(() => saveRecordAndRefresh('action', action, base));
  };
  const handleDeleteAction = (id: string, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    return deleteRecord('action', id, revision, host);
  };
  const handleSetActionEnabled = (id: string, enabled: boolean, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    return setRecordEnabled('action', id, enabled, revision, host);
  };
  const handleTestAction = (action: LiveAction, trigger?: string): void => {
    testRecord('action', action, trigger);
  };
  const handleSaveEvent = (event: LiveEvent, base: AutomationWriteBase): void => {
    void mutate(() => saveRecordAndRefresh('event', event, base));
  };
  const handleDeleteEvent = (id: string, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    return deleteRecord('event', id, revision, host);
  };
  const handleSetEventEnabled = (id: string, enabled: boolean, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    return setRecordEnabled('event', id, enabled, revision, host);
  };
  const handleTestEvent = (event: LiveEvent): void => {
    testRecord('event', event);
  };
  const handleApplyRuleTemplate = async (actions: LiveAction[], event: LiveEvent): Promise<void> => {
    if (persistent.value !== false) {
      const context = library.snapshot.value;
      if (!context) throw new ControlCallError('capability_unavailable', 'Read the current library first.');
      await library.request({ operation: 'apply-template', entry: { actions, event }, profileId: context.catalog.activeId }, context);
      await refresh(); return;
    }
    await mutate(async () => {
      for (const action of actions) {
        await control.call('automation.create', { kind: 'action', record: action }, checkedOptions(persistent.value === false ? null : '0'));
      }
      await control.call('automation.create', { kind: 'event', record: event }, checkedOptions(persistent.value === false ? null : '0'));
    });
  };

  const createProfileRecords = async (
    entries: Array<{ actions: LiveAction[]; event: LiveEvent }>,
  ): Promise<void> => {
    if (persistent.value !== false) throw new ControlCallError('capability_unavailable', 'Persistent profile graphs require the native atomic library.');
    for (const entry of entries) {
      for (const action of entry.actions) {
        await control.call('automation.create', { kind: 'action', record: action }, checkedOptions(persistent.value === false ? null : '0'));
      }
      await control.call('automation.create', { kind: 'event', record: entry.event }, checkedOptions(persistent.value === false ? null : '0'));
    }
  };

  const handleRequestHotkeyAccess = (): void => {
    if (hotkeyAccessPending.value) return;
    hotkeyAccessPending.value = true;
    clearBehaviorError();
    void control
      .call<{ granted: boolean; message: string }>('system.requestInputAccess', {})
      .then((result) => {
        if (disposed) return;
        if (result.granted) return refresh();
        behaviorError.value = result.message || 'Access was not granted.';
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        behaviorError.value = errorMessage(failure);
      })
      .finally(() => {
        hotkeyAccessPending.value = false;
      });
  };

  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    owner.dispose();
    ++refreshRevision;
    if (refreshTimer !== null) clearTimeout(refreshTimer);
    refreshTimer = null;
    runBuffer.dispose();
  };

  return {
    diagnostics: () => ({ timers: Number(refreshTimer !== null) + runBuffer.diagnostics().timers, disposed }),
    dispose,
    setPresentationActive,
    behavior,
    behaviorVersions,
    persistent,
    sourceHost,
    revisionFor,
    readRecord,
    deleteRecord,
    setRecordEnabled,
    behaviorRuns,
    behaviorTestRuns,
    behaviorError,
    hotkeyStatus,
    lastHotkeyEvent,
    hotkeyAccessPending,
    handleRequestHotkeyAccess,
    pluginPages,
    pluginUis,
    clearBehaviorError,
    handleSaveAction,
    handleDeleteAction,
    handleSetActionEnabled,
    handleTestAction,
    handleSaveEvent,
    handleDeleteEvent,
    handleSetEventEnabled,
    handleTestEvent,
    handleFireEvent,
    handleAnalyzeScript,
    handleApplyRuleTemplate,
    createProfileRecords,
    saveRecordAndRefresh,
    refresh,
  };
}
