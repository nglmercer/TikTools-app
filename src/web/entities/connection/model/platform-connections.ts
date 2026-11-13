import { computed, ref } from 'vue';
import type { ConnectionSnapshot, PlatformDescriptor, PlatformId } from '../../../../automation/contracts/generated/automation-events.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';

export function createPlatformConnections(control: ControlClient) {
  const platforms = ref<PlatformDescriptor[]>([]);
  const connections = ref<ConnectionSnapshot[]>([]);
  const busy = ref(false);
  const error = ref('');
  let revision = 0;
  let disposed = false;
  const removed = new Set<string>();
  const off = control.onTopic<{ connection: ConnectionSnapshot }>('live.platform.connection', ({ connection }) => {
    if (removed.has(connection.connectionId)) return;
    revision += 1;
    const index = connections.value.findIndex((item) => item.connectionId === connection.connectionId);
    if (index < 0) connections.value.push(connection);
    else connections.value[index] = connection;
  });
  async function refresh(): Promise<void> {
    const request = revision;
    try {
      const [descriptors, snapshots] = await Promise.all([
        control.call<PlatformDescriptor[]>('live.platforms', {}),
        control.call<ConnectionSnapshot[]>('live.connections', {}),
      ]);
      if (disposed) return;
      if (!Array.isArray(descriptors) || !Array.isArray(snapshots)) throw new Error('Invalid platform connection response');
      platforms.value = descriptors;
      if (request === revision) connections.value = snapshots;
    } catch (failure) { if (!disposed) error.value = errorMessage(failure); }
  }
  async function connect(platform: PlatformId, target: string, token: string): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    error.value = '';
    let credentialRef: string | undefined;
    try {
      if (token) {
        const saved = await control.call<{ credentialRef: string }>('live.platform.credentials.store', { platform, token });
        credentialRef = saved.credentialRef;
      }
      const snapshot = await control.call<ConnectionSnapshot>('live.platform.connect', { platform, target: target.trim(), credentialRef, roomId: null });
      if (disposed) return;
      if (!connections.value.some((item) => item.connectionId === snapshot.connectionId)) connections.value.push(snapshot);
    } catch (failure) { if (!disposed) error.value = errorMessage(failure); }
    finally {
      // The adapter already owns a private in-memory copy. Avoid retaining
      // abandoned tokens or accumulating unused files after every reconnect.
      if (credentialRef) {
        try { await control.call('live.platform.credentials.delete', { platform, credentialRef }); }
        catch (failure) { if (!disposed) error.value = errorMessage(failure); }
      }
      busy.value = false;
    }
  }
  async function disconnect(connectionId: string): Promise<void> {
    error.value = '';
    try {
      await control.call('live.platform.disconnect', { connectionId });
      removed.add(connectionId);
      if (removed.size > 1024) removed.delete(removed.values().next().value!);
      revision += 1;
      connections.value = connections.value.filter((item) => item.connectionId !== connectionId);
    } catch (failure) { if (!disposed) error.value = errorMessage(failure); }
  }
  return { platforms, connections, busy, error, refresh, connect, disconnect,
    additionalPlatforms: computed(() => platforms.value.filter((item) => item.platform !== 'tiktok')),
    dispose() { disposed = true; revision += 1; off(); },
  };
}
