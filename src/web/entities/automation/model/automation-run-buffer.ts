import { shallowRef } from 'vue';
import type { BehaviorRun } from '../../../../automation/behavior/types.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';

export interface AutomationRunsResult {
  runs: BehaviorRun[];
}

/** One bounded run buffer; callers share the subscription owner's client. */
export function createAutomationRunBuffer(control: ControlClient, isDisposed: () => boolean) {
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
    if (isDisposed() || !presentationActive || pendingRuns.size === 0) return;
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
  return {
    behaviorRuns, applyRuns, setPresentationActive,
    revision: () => runsRevision,
    diagnostics: () => ({ timers: Number(runsTimer !== null) }),
    dispose: () => {
      if (runsTimer !== null) clearTimeout(runsTimer);
      runsTimer = null;
      pendingRuns.clear();
    },
  };
}
