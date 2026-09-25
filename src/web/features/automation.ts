import { computed, ref, type ComputedRef } from 'vue';

import type { AutomationEventType, AutomationScriptAnalysis } from '../../automation/types.ts';
import type {
  BehaviorRun,
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
  PluginPageDescriptor,
} from '../../automation/behavior/types.ts';
import {
  applySnapshotContributions,
  syncAutocompletePluginStates,
} from '../../automation/autocomplete-registry.ts';
import { setPluginEventTypes } from '../../automation/event-registry.ts';
import {
  mergePluginAutocomplete,
  mergePluginPages,
  mergePluginUis,
} from '../../automation/plugins/declarative.ts';
import type { HotkeyStatusData } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError, errorMessage } from '../platform/control-client.ts';
import { setPluginTranslations } from '../i18n.ts';

type AutomationKind = 'event' | 'action';

export interface AutomationRunsResult {
  runs: BehaviorRun[];
}

export interface LastHotkeyEvent {
  key: string;
  modifiers: string;
  backend: string;
  at: number;
}

/** Behavior snapshot, records, runs, and the workflow graph editor. */
export function useAutomation(control: ControlClient) {
  const behavior = ref<BehaviorSnapshot>({
    actions: [],
    events: [],
    plugins: [],
    actionTypes: [],
    translations: {},
  });
  const behaviorRuns = ref<BehaviorRun[]>([]);
  const behaviorTestRuns = ref<BehaviorRun[]>([]);
  const behaviorError = ref('');
  const hotkeyStatus = ref<HotkeyStatusData | null>(null);
  const lastHotkeyEvent = ref<LastHotkeyEvent | null>(null);
  const hotkeyAccessPending = ref(false);

  /** Plugin configuration pages from the behavior snapshot, validated. */
  const pluginPages: ComputedRef<PluginPageDescriptor[]> = computed(() =>
    mergePluginPages(behavior.value.pluginPages),
  );

  /** Typed `ui` descriptors from the behavior snapshot, re-validated. */
  const pluginUis = computed(() => mergePluginUis(behavior.value.pluginUis));

  const clearBehaviorError = (): void => {
    behaviorError.value = '';
  };

  const applySnapshot = (snapshot: BehaviorSnapshot): void => {
    setPluginTranslations(snapshot.translations);
    setPluginEventTypes(snapshot.eventTypes ?? []);
    // Seed plugin-declared autocomplete contributions (manifest
    // `autocomplete` sections, shipped only while their plugin is installed,
    // enabled, and available) and gate undeclared provider-namespace paths
    // on plugin state. Absent on old hosts: nothing is seeded and
    // suggestions keep their static behavior.
    applySnapshotContributions(mergePluginAutocomplete(snapshot.autocompleteContributions));
    syncAutocompletePluginStates(
      snapshot.plugins.map((plugin) => ({
        id: plugin.descriptor.id,
        installed: plugin.installed,
        enabled: plugin.enabled,
        available: plugin.available,
      })),
    );
    behavior.value = snapshot;
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
  // Live run updates: the authoritative `automation.runs.changed` domain
  // topic is primary, the legacy `behavior-runs` push stays as compat.
  // Both write the same ref, so no manual refresh is ever required.
  control.onPush('behavior-runs', (message) => {
    if (message.type !== 'behavior-runs') return;
    behaviorRuns.value = message.runs;
  });
  control.onTopic<{ runs: BehaviorRun[] }>('automation.runs.changed', (data) => {
    if (!data || !Array.isArray(data.runs)) return;
    behaviorRuns.value = data.runs;
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
  control.onTopic('workflow.changed', () => {
    void refresh().catch((failure: unknown) => {
      console.warn(`automation refresh failed: ${errorMessage(failure)}`);
    });
  });

  const refresh = async (): Promise<void> => {
    try {
      const [snapshot, runs] = await Promise.all([
        control.call<BehaviorSnapshot>('automation.snapshot', {}),
        control.call<AutomationRunsResult>('automation.runs', {}),
      ]);
      applySnapshot(snapshot);
      behaviorRuns.value = runs.runs;
    } catch (failure) {
      behaviorError.value = errorMessage(failure);
    }
  };

  const mutate = async (work: () => Promise<unknown>): Promise<void> => {
    clearBehaviorError();
    try {
      await work();
    } catch (failure) {
      behaviorError.value = errorMessage(failure);
      return;
    }
    await refresh();
  };

  /** Legacy save is an upsert: update when the record exists, create
   * (keeping the caller id) when it does not. */
  const saveRecord = (
    kind: AutomationKind,
    record: LiveAction | LiveEvent,
  ): Promise<unknown> => {
    const id = (record as { id?: unknown }).id;
    if (typeof id === 'string' && id.trim() !== '') {
      return control.call('automation.update', { id, kind, record }).catch((failure: unknown) => {
        if (failure instanceof ControlCallError && failure.code === 'automation_not_found') {
          return control.call('automation.create', { kind, record });
        }
        throw failure;
      });
    }
    return control.call('automation.create', { kind, record });
  };

  const setRecordEnabled = (kind: AutomationKind, id: string, enabled: boolean): void => {
    void mutate(() =>
      control.call(enabled ? 'automation.enable' : 'automation.disable', { id, kind }),
    );
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
        behaviorTestRuns.value = [run];
      })
      .catch((failure: unknown) => {
        behaviorError.value = errorMessage(failure);
      });
  };

  const handleSaveAction = (action: LiveAction): void => {
    void mutate(() => saveRecord('action', action));
  };
  const handleDeleteAction = (id: string): void => {
    void mutate(() => control.call('automation.delete', { id, kind: 'action' }));
  };
  const handleSetActionEnabled = (id: string, enabled: boolean): void => {
    setRecordEnabled('action', id, enabled);
  };
  const handleTestAction = (action: LiveAction, trigger?: string): void => {
    testRecord('action', action, trigger);
  };
  const handleSaveEvent = (event: LiveEvent): void => {
    void mutate(() => saveRecord('event', event));
  };
  const handleDeleteEvent = (id: string): void => {
    void mutate(() => control.call('automation.delete', { id, kind: 'event' }));
  };
  const handleSetEventEnabled = (id: string, enabled: boolean): void => {
    setRecordEnabled('event', id, enabled);
  };
  const handleTestEvent = (event: LiveEvent): void => {
    testRecord('event', event);
  };

  /** One-click seat access: the host probes, then polkit-prompts at most
   * once. Guarded against double clicks; the polkit dialog can sit open
   * a while, so the button stays disabled until the call settles. A
   * dismissed prompt is an Ok result with `granted: false`, not an
   * error, so the message surfaces explicitly instead of vanishing. */
  const handleRequestHotkeyAccess = (): void => {
    if (hotkeyAccessPending.value) return;
    hotkeyAccessPending.value = true;
    clearBehaviorError();
    void control
      .call<{ granted: boolean; message: string }>('system.requestInputAccess', {})
      .then((result) => {
        if (result.granted) return refresh();
        behaviorError.value = result.message || 'Access was not granted.';
      })
      .catch((failure: unknown) => {
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

  return {
    behavior,
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
    handleAnalyzeScript,
    refresh,
  };
}
