import type { CollaborationStatus } from '@/entities/collaboration/api/types.ts';
export function collaborationStatus(): CollaborationStatus {
  return {
    hostId: 'owner-host', hostInstanceId: 'owner-instance', configurationRevision: '1', activeSessions: 0, pendingRequests: 0,
    configuration: { enabled: true, bindAddress: '127.0.0.1:8788', advertisedOrigin: 'https://pc-a.example:8788',
      allowedOrigins: [], tlsCertificate: '/host/cert.pem', tlsPrivateKey: '/host/key.pem', tlsProxy: false,
      limits: { connections: 8, connectionsPerPrincipal: 4, frameBytes: 1048576, paramsBytes: 262144, pendingCalls: 32, requestsPerSecond: 30, requestBurst: 60, commandIdsPerPrincipal: 100000 } },
    gateway: { available: true, running: true, connections: 0, configurationRevision: '1', address: '127.0.0.1:8788',
      lastError: null, certificateFingerprint: 'a'.repeat(64), sessionConnections: {} },
  };
}
export function emptyCollaborationPage(method: string) {
  const key = method.includes('.invites.') ? 'invitations' : method.includes('.audit.') ? 'entries' : method.split('.')[1]!;
  return { [key]: [], nextAfter: method.includes('.audit.') ? 0 : null };
}
