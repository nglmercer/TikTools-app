import { computed, ref, type ComputedRef } from 'vue';

import type { AutomationEventType, AutomationScriptAnalysis } from '../../automation/types.ts';
import type {
  BehaviorRun,
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
  PluginPageDescriptor,
} from '../../automation/behavior/types.ts';
import { setPluginEventTypes } from '../../automation/event-registry.ts';
import { mergePluginPages } from '../../automation/plugins/declarative.ts';
import type { HotkeyStatusData } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError, errorMessage } from '../platform/control-client.ts';
import { setPluginTranslations } from '../i18n.ts';

type AutomationKind = 'event' | 'action';

export interface AutomationRunsResult {
  runs: BehaviorRun[];
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

  /** Plugin configuration pages from the behavior snapshot, validated. */
  const pluginPages: ComputedRef<PluginPageDescriptor[]> = computed(() =>
    mergePluginPages(behavior.value.pluginPages),
  );

  const clearBehaviorError = (): void => {
    behaviorError.value = '';
  };

  const applySnapshot = (snapshot: BehaviorSnapshot): void => {
    setPluginTranslations(snapshot.translations);
    setPluginEventTypes(snapshot.eventTypes ?? []);
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
    pluginPages,
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
