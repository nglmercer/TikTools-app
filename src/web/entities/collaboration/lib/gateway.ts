import type { CollaborationStatus, GatewayStatus } from '../api/types.ts';

export function activeGateway(status: CollaborationStatus): GatewayStatus {
  return status.peerGateway?.available ? status.peerGateway : status.gateway;
}
