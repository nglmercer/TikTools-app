import type { Configuration, Grant, PairingRequest } from '@/entities/collaboration/api/types.ts';

/** Owner actions remain pending until the composition root confirms them. */
export interface CollaborationConfirmation {
  action: 'disable' | 'approve' | 'reject' | 'session' | 'principal' | 'invite' | 'grant';
  id?: string;
  grant?: Grant;
  revision?: string;
  request?: PairingRequest;
  configuration?: Configuration;
}
