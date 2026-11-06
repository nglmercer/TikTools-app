import type { ControlClient } from '@/shared/api/control-client.ts';
import type { UpdateChannel, UpdateStatus } from './updater-types.ts';

export const updaterApi = {
  status: (client: ControlClient) => client.call<UpdateStatus>('app.update.status', {}),
  check: (client: ControlClient, channel: UpdateChannel) => client.call<UpdateStatus>('app.update.check', { channel }),
  download: (client: ControlClient) => client.call<UpdateStatus>('app.update.download', {}),
};
