import { ref, type Ref } from 'vue';
import type { AutomationEventType, AutomationScriptAnalysis } from '../../../../automation/types.ts';
import type { BehaviorRun, LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { AutomationKind } from './record-versions.ts';

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

export function createAutomationExecution(control: ControlClient, context: {
  isDisposed: () => boolean;
  error: Ref<string>;
  clearError: () => void;
}) {
  const behaviorTestRuns = ref<BehaviorRun[]>([]);
  const testRecord = (
    kind: AutomationKind,
    record: LiveAction | LiveEvent,
    trigger?: string,
  ): void => {
    context.clearError();
    behaviorTestRuns.value = [];
    void control
      .call<BehaviorRun>('automation.test', { record, kind, trigger })
      .then((run) => {
        if (context.isDisposed()) return;
        behaviorTestRuns.value = [run];
      })
      .catch((failure: unknown) => {
        if (context.isDisposed()) return;
        context.error.value = errorMessage(failure);
      });
  };

  const handleFireEvent = (event: LiveEvent): void => {
    context.clearError();
    behaviorTestRuns.value = [];
    void control
      .call<AutomationFireResult>('automation.fire', { trigger: event.trigger, record: event })
      .then((result) => {
        if (context.isDisposed()) return;
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
        if (context.isDisposed()) return;
        context.error.value = errorMessage(failure);
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

  return { behaviorTestRuns, testRecord, handleFireEvent, handleAnalyzeScript };
}
