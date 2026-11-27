import { describe, expect, it } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createLiveMonitorState } from '../../../../../../src/web/entities/connection/model/live-monitor-state.ts';

const status = {
  enabled: true,
  uniqueId: 'alice',
  phase: 'waiting',
  pollIntervalSeconds: 30,
  checks: 0,
  failures: 0,
  lastCheckedAt: null,
  nextCheckAt: null,
  lastError: null,
};
describe('native monitor configuration', () => {
  it('starts and stops exclusively through the control API', async () => {
    const { client, calls } = createFakeControlClient((method) => method === 'live.monitor.stop' ? { ...status, enabled: false, phase: 'stopped' } : status);
    const monitor = createLiveMonitorState(client);
    await monitor.startMonitor('alice', 'sessionid=secret');
    expect(calls[0]).toEqual({ method: 'live.monitor.start', params: { uniqueId: 'alice', sessionCookie: 'sessionid=secret', pollIntervalSeconds: 30 } });
    expect(monitor.monitorStatus.value?.enabled).toBe(true);
    await monitor.stopMonitor();
    expect(calls[1]?.method).toBe('live.monitor.stop');
    expect(monitor.monitorStatus.value?.phase).toBe('stopped');
    monitor.disposeMonitor();
  });
  it('reports errors without fabricating a successful monitor', async () => {
    const { client } = createFakeControlClient(() => { throw new Error('unsupported'); });
    const monitor = createLiveMonitorState(client);
    await monitor.startMonitor('alice', '');
    expect(monitor.monitorStatus.value).toBeNull();
    expect(monitor.monitorError.value).toContain('unsupported');
    expect(monitor.monitorBusy.value).toBe(false);
    monitor.disposeMonitor();
  });
});
