import { ownControlSubscriptions } from '@/shared/api/control-owner.ts';
import { computed, ref, shallowRef, type ComputedRef } from 'vue';

import type { AutomationEventType, AutomationScriptAnalysis } from '../../../../automation/types.ts';
import type {
  BehaviorRun,
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
  PluginPageDescriptor,
} from '../../../../automation/behavior/types.ts';
import {
  applySnapshotContributions,
  syncAutocompletePluginStates,
} from '../../../../automation/autocomplete-registry.ts';
import { setPluginEventTypes } from '../../../../automation/event-registry.ts';
import {
  mergePluginAutocomplete,
  mergePluginPages,
  mergePluginUis,
} from '../../../../automation/plugins/declarative.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { callWithMeta, ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { assertRecord, automationVersions, recordKey, recordRevision, validRevision,
  type AutomationKind, type AutomationRecord, type AutomationRecordVersion, type AutomationRevision, type AutomationWriteBase,
} from './record-versions.ts';
import { mergePluginMessages } from '@/shared/i18n/i18n.ts';

export interface AutomationRunsResult {
  runs: BehaviorRun[];
}

export interface AutomationFireResult {
  trigger: string;
  eventSource: 'live' | 'sample' | 'custom';
  pinned: string[];
  matched: number;
  draftMatched: boolean;
  status: 'ok' | 'error';
  summary: string;
  durationMs: number;
}

export interface LastHotkeyEvent {
  key: string;
  modifiers: string;
  backend: string;
  at: number;
}

export function useAutomation(control: ControlClient) {
  const owner = ownControlSubscriptions(control);
  control = owner.client;
  let disposed = false;

  const initialSnapshot: BehaviorSnapshot = {
    actions: [],
    events: [],
    plugins: [],
    actionTypes: [],
    translations: {},
  };
  const projection = shallowRef<{ snapshot: BehaviorSnapshot; versions: ReadonlyMap<string, string>; persistent: boolean | null }>({
    snapshot: initialSnapshot, versions: new Map(), persistent: null,
  });
  const behavior = computed({
    get: () => projection.value.snapshot,
    // Legacy test/embedding assignment is a body-only projection, never a
    // source of checked-write authority for an unrelated version map.
    set: (snapshot: BehaviorSnapshot) => { projection.value = { snapshot, versions: new Map(), persistent: null }; },
  });
  const behaviorVersions = computed(() => projection.value.versions);
  const persistent = computed(() => projection.value.persistent);
  const revisionFor = (kind: AutomationKind, id: string): AutomationRevision | undefined =>
    behaviorVersions.value.get(recordKey(kind, id)) ?? (persistent.value === false ? null : undefined);
  const behaviorRuns = shallowRef<BehaviorRun[]>([]);
  let runsRevision = 0;
  let presentationActive = true;
  const pendingRuns = new Map<string, BehaviorRun>();
  let runsTimer: ReturnType<typeof setTimeout> | null = null;
  const applyRuns = (runs: BehaviorRun[]): void => {
    if (!presentationActive) return;
    ++runsRevision;
    pendingRuns.clear();
    behaviorRuns.value = runs.slice(0, 60);
  };
  const commitRuns = (): void => {
    runsTimer = null;
    if (disposed || !presentationActive || pendingRuns.size === 0) return;
    const incoming = [...pendingRuns.values()].reverse();
    const ids = new Set(pendingRuns.keys());
    applyRuns([...incoming, ...behaviorRuns.value.filter((run) => !ids.has(run.id))]);
  };
  control.onTopic<{ run: BehaviorRun }>('automation.run.completed', (data) => {
    if (!presentationActive || !data?.run?.id) return;
    ++runsRevision;
    pendingRuns.delete(data.run.id);
    pendingRuns.set(data.run.id, data.run);
    if (pendingRuns.size > 60) {
      const oldest = pendingRuns.keys().next().value;
      if (oldest !== undefined) pendingRuns.delete(oldest);
    }
    if (runsTimer === null) runsTimer = setTimeout(commitRuns, 100);
  });
  const setPresentationActive = (active: boolean): void => {
    presentationActive = active;
    if (!active) {
      if (runsTimer !== null) clearTimeout(runsTimer);
      runsTimer = null;
      pendingRuns.clear();
    }
  };
  const behaviorTestRuns = ref<BehaviorRun[]>([]);
  const behaviorError = ref('');
  const hotkeyStatus = ref<HotkeyStatusData | null>(null);
  const lastHotkeyEvent = ref<LastHotkeyEvent | null>(null);
  const hotkeyAccessPending = ref(false);

  const pluginPages: ComputedRef<PluginPageDescriptor[]> = computed(() =>
    mergePluginPages(behavior.value.pluginPages),
  );

  const pluginUis = computed(() => mergePluginUis(behavior.value.pluginUis));

  const clearBehaviorError = (): void => {
    behaviorError.value = '';
  };

  const applySnapshot = (snapshot: BehaviorSnapshot, versions: ReadonlyMap<string, string>, persistence: boolean | null): void => {
    if (disposed) return;
    mergePluginMessages(snapshot.translations ?? {});
    setPluginEventTypes(snapshot.eventTypes ?? []);
    applySnapshotContributions(mergePluginAutocomplete(snapshot.autocompleteContributions));
    syncAutocompletePluginStates(
      snapshot.plugins.map((plugin) => ({
        id: plugin.descriptor.id,
        installed: plugin.installed,
        enabled: plugin.enabled,
        available: plugin.available,
      })),
    );
    projection.value = { snapshot, versions, persistent: persistence };
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
  control.onPush('behavior-runs', (message) => {
    if (message.type !== 'behavior-runs') return;
    applyRuns(message.runs);
  });
  control.onTopic<{ runs: BehaviorRun[] }>('automation.runs.changed', (data) => {
    if (!data || !Array.isArray(data.runs)) return;
    applyRuns(data.runs);
  });
  control.onTopic<AutomationRunsResult>('ui.automation.runs', (data) => {
    if (!data || !Array.isArray(data.runs)) return;
    applyRuns(data.runs);
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
    const runsAtStart = runsRevision;
    try {
      const [snapshot, runs, info] = await Promise.all([
        callWithMeta<BehaviorSnapshot>(control, 'automation.snapshot', {}),
        control.call<AutomationRunsResult>('automation.runs', {}),
        control.call<{ features?: { persistence?: boolean } }>('system.info', {}).catch(() => null),
      ]);
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
      applySnapshot(snapshot.result, versions, typeof info?.features?.persistence === 'boolean' ? info.features.persistence : null);
      if (runsAtStart === runsRevision) applyRuns(runs.runs);
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

  const checkedOptions = (revision: AutomationRevision) => {
    if (disposed) throw new ControlCallError('detached', 'Automation state was disposed.');
    if (revision === null) {
      if (persistent.value !== false) throw new ControlCallError('capability_unavailable', 'Read the current record revision before writing.');
      return undefined;
    }
    if (!validRevision(revision)) throw new ControlCallError('invalid_params', 'Invalid automation base revision.');
    return { meta: { expectedRevision: revision } };
  };
  const readRecord = async (kind: AutomationKind, id: string): Promise<AutomationRecordVersion> => {
    if (disposed) throw new ControlCallError('detached', 'Automation state was disposed.');
    try {
      const response = await callWithMeta<AutomationRecord>(control, 'automation.get', { id, kind });
      assertRecord(response.result, kind, id);
      return { record: response.result, revision: recordRevision(response.meta, kind, id, persistent.value === false) };
    } catch (failure) {
      if (!(failure instanceof ControlCallError) || failure.code !== 'automation_not_found') throw failure;
      const data = failure.data as { resourceVersions?: unknown } | undefined;
      const meta = Array.isArray(data?.resourceVersions) ? { resourceVersions: data.resourceVersions } : undefined;
      return { record: null, revision: recordRevision(meta, kind, id, persistent.value === false) };
    }
  };
  const mutateVersioned = async <T>(work: () => Promise<T>): Promise<T> => {
    // An older snapshot read cannot rewind a successful write's refresh.
    ++refreshRevision;
    // The page owns its failure/draft scope; a late rejection must not put an
    // old record's error into a different editor. Legacy handlers report below.
    const result = await work();
    if (!disposed) await refresh();
    return result;
  };
  const deleteRecord = (kind: AutomationKind, id: string, revision: AutomationRevision): Promise<void> => {
    const options = checkedOptions(revision);
    return mutateVersioned(async () => {
      const response = await callWithMeta<{ deleted: boolean }>(control, 'automation.delete', { id, kind }, options);
      const after = recordRevision(response.meta, kind, id, revision === null);
      if (revision !== null && after !== (response.result.deleted ? String(BigInt(revision) + 1n) : revision)) {
        throw new ControlCallError('protocol', 'Unexpected automation delete revision.');
      }
    });
  };
  const setRecordEnabled = (kind: AutomationKind, id: string, enabled: boolean, revision: AutomationRevision): Promise<void> => {
    const options = checkedOptions(revision);
    return mutateVersioned(async () => {
      const response = await callWithMeta<AutomationRecord>(control, enabled ? 'automation.enable' : 'automation.disable', { id, kind }, options);
      assertRecord(response.result, kind, id);
      const after = recordRevision(response.meta, kind, id, revision === null);
      if (response.result.enabled !== enabled || (revision !== null && after !== String(BigInt(revision) + 1n))) {
        throw new ControlCallError('protocol', 'Unexpected automation enablement acknowledgement.');
      }
    });
  };

  const testRecord = (
    kind: AutomationKind,
    record: LiveAction | LiveEvent,
    trigger?: string,
  ): void => {
    clearBehaviorError();
    behaviorTestRuns.value = [];
    void control
      .call<BehaviorRun>('automation.test', { record, kind, trigger })
      .then((run) => {
        if (disposed) return;
        behaviorTestRuns.value = [run];
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        behaviorError.value = errorMessage(failure);
      });
  };

  const handleSaveAction = (action: LiveAction, base: AutomationWriteBase): void => {
    void mutate(() => saveRecordAndRefresh('action', action, base));
  };
  const handleDeleteAction = (id: string, revision: AutomationRevision): Promise<void> => {
    return deleteRecord('action', id, revision);
  };
  const handleSetActionEnabled = (id: string, enabled: boolean, revision: AutomationRevision): Promise<void> => {
    return setRecordEnabled('action', id, enabled, revision);
  };
  const handleTestAction = (action: LiveAction, trigger?: string): void => {
    testRecord('action', action, trigger);
  };
  const handleSaveEvent = (event: LiveEvent, base: AutomationWriteBase): void => {
    void mutate(() => saveRecordAndRefresh('event', event, base));
  };
  const handleDeleteEvent = (id: string, revision: AutomationRevision): Promise<void> => {
    return deleteRecord('event', id, revision);
  };
  const handleSetEventEnabled = (id: string, enabled: boolean, revision: AutomationRevision): Promise<void> => {
    return setRecordEnabled('event', id, enabled, revision);
  };
  const handleTestEvent = (event: LiveEvent): void => {
    testRecord('event', event);
  };
  const handleFireEvent = (event: LiveEvent): void => {
    clearBehaviorError();
    behaviorTestRuns.value = [];
    void control
      .call<AutomationFireResult>('automation.fire', { trigger: event.trigger, record: event })
      .then((result) => {
        if (disposed) return;
        behaviorTestRuns.value = [
          {
            id: `fire-${Date.now()}`,
            at: Date.now(),
            status: result.status,
            eventName: event.trigger,
            actionName: event.name,
            summary: result.summary,
            durationMs: result.durationMs,
            test: true,
            logs: [],
            eventSource: result.eventSource,
            ...(result.status === 'error' ? { error: result.summary } : {}),
          },
        ];
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        behaviorError.value = errorMessage(failure);
      });
  };

  const saveRecordAndRefresh = async (
    kind: AutomationKind,
    record: AutomationRecord,
    base: AutomationWriteBase,
  ): Promise<AutomationRecordVersion> => {
    const options = checkedOptions(base.revision);
    return mutateVersioned(async () => {
      const method = base.create ? 'automation.create' : 'automation.update';
      const response = await callWithMeta<AutomationRecord>(control, method,
        base.create ? { kind, record } : { id: record.id, kind, record }, options);
      assertRecord(response.result, kind, record.id);
      const revision = recordRevision(response.meta, kind, record.id, base.revision === null);
      if (base.revision !== null && revision !== String(BigInt(base.revision) + 1n)) {
        throw new ControlCallError('protocol', 'Unexpected automation save revision.');
      }
      return { record: response.result, revision };
    });
  };

  const handleApplyRuleTemplate = (actions: LiveAction[], event: LiveEvent): void => {
    void mutate(async () => {
      for (const action of actions) {
        await control.call('automation.create', { kind: 'action', record: action }, checkedOptions(persistent.value === false ? null : '0'));
      }
      await control.call('automation.create', { kind: 'event', record: event }, checkedOptions(persistent.value === false ? null : '0'));
    });
  };

  const createProfileRecords = async (
    entries: Array<{ actions: LiveAction[]; event: LiveEvent }>,
  ): Promise<void> => {
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

  const handleAnalyzeScript = (
    nodeId: string,
    source: string,
    offset: number,
    eventType?: AutomationEventType,
  ): Promise<AutomationScriptAnalysis> =>
    control.call<AutomationScriptAnalysis>('automation.script.analyze', {
      nodeId,
      source,
      offset,
      eventType,
    });

  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    owner.dispose();
    ++refreshRevision;
    if (refreshTimer !== null) clearTimeout(refreshTimer);
    refreshTimer = null;
    if (runsTimer !== null) clearTimeout(runsTimer);
    runsTimer = null;
    pendingRuns.clear();
  };

  return {
    diagnostics: () => ({ timers: Number(refreshTimer !== null) + Number(runsTimer !== null), disposed }),
    dispose,
    setPresentationActive,
    behavior,
    behaviorVersions,
    persistent,
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
