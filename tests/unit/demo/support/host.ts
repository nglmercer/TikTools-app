import { createControlClient } from '@/shared/api/control-client.ts';
import { createDemoHost } from '../../../../src/web/demo/runtime/host.ts';
import type { ScenarioId } from '../../../../src/web/demo/scenarios.ts';

/**
 * A demo host with an attached control client, the wiring most demo tests start from.
 * `dispose` detaches the client before tearing the host down.
 */
export function createDemoFixture(scenario: ScenarioId = 'connection', options: { timeoutMs?: number } = {}) {
  const host = createDemoHost(scenario);
  const client = createControlClient({ transport: host.transport, ...options });
  client.attach();
  return {
    host,
    client,
    dispose() {
      client.detach();
      host.dispose();
    },
  };
}
