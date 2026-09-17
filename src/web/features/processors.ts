import { ref } from 'vue';

import type { AutomationEvent, JsonObject } from '../../automation/types.ts';
import type { HostMessage, ProcessorStatusEntry } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';

export interface ProcessorTestResult {
  pluginId: string;
  processorId: string;
  ok: boolean;
  durationMs: number;
  result: JsonObject;
  error?: string | null;
}

export interface ProcessorStatusResult {
  processors: ProcessorStatusEntry[];
}

/** Event processors: status panel and dry-run tests. */
export function useProcessors(control: ControlClient) {
  const processors = ref<ProcessorStatusEntry[]>([]);
  const processorTest = ref<Extract<HostMessage, { type: 'processor-test-result' }> | null>(null);

  const refresh = async (): Promise<void> => {
    try {
      const result = await control.call<ProcessorStatusResult>('processors.status', {});
      processors.value = result.processors;
    } catch (failure) {
      console.warn(`processors.status failed: ${errorMessage(failure)}`);
    }
  };

  const handleGetProcessorStatus = (): void => {
    void refresh();
  };

  const handleTestProcessor = (
    pluginId: string,
    processorId: string,
    event: AutomationEvent,
  ): void => {
    processorTest.value = null;
    void control
      .call<ProcessorTestResult>('processors.test', { pluginId, processorId, event })
      .then((result) => {
        processorTest.value = {
          type: 'processor-test-result',
          pluginId: result.pluginId,
          processorId: result.processorId,
          ok: result.ok,
          durationMs: result.durationMs,
          result: result.result,
          error: result.error ?? undefined,
        };
        // Tests feed the same health/metrics counters, so refresh the panel
        // instead of leaving the pre-test snapshot on screen.
        void refresh();
      })
      .catch((failure: unknown) => {
        processorTest.value = {
          type: 'processor-test-result',
          pluginId,
          processorId,
          ok: false,
          durationMs: 0,
          result: {},
          error: errorMessage(failure),
        };
      });
  };

  return {
    processors,
    processorTest,
    handleGetProcessorStatus,
    handleTestProcessor,
    refresh,
  };
}
