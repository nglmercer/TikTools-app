export const capabilities = [
  'live.read', 'points.read', 'points.adjust', 'rankings.read', 'rankings.manage', 'competitions.read', 'competitions.manage',
  'workflows.read', 'workflows.edit', 'workflows.toggle', 'automation.read', 'automation.edit', 'automation.toggle',
  'plugins.read', 'plugins.toggle', 'plugins.settings.read', 'plugins.settings.write', 'plugins.action.execute', 'widgets.read', 'widgets.configure', 'commands.read',
  'http.request', 'audio.play', 'events.emit', 'scripts.execute',
] as const;
export type Capability = typeof capabilities[number];
export const resourceKinds = ['live', 'viewer', 'ranking', 'competition', 'workflow', 'automation', 'plugin', 'widget'] as const;
export type ResourceKind = typeof resourceKinds[number];
export interface HttpEffectGrant {
  origin: string; methods: string[]; paths: string[]; queryFields: string[]; headerNames: string[];
  renderFields: string[]; bodyFields: string[]; allowTextBody: boolean; maxBodyBytes: number;
  maxResponseBytes: number; allowPrivateNetwork: boolean;
}
export interface AudioEffectGrant { fileRefs: string[]; output: 'host-default'; maxVolume: number; overlaps: ('allow' | 'restart' | 'drop')[] }
export interface EffectGrants { http: HttpEffectGrant[]; audio: AudioEffectGrant[]; emitTopics: string[] }
/** Reusable policy for supported saved automation; resource and effect limits still apply. */
export interface AutomationDelegation { effects: ('log' | 'delay' | 'points' | 'http' | 'audio' | 'emit' | 'script')[]; allowDynamicTargets: boolean; allowOwnerEdits?: boolean; allowOwnerSource?: boolean }
export interface Grant {
  capabilities: Capability[];
  resources: Partial<Record<ResourceKind, string[]>>;
  settingsFields: Record<string, string[]>;
  actions: Record<string, string[]>;
  maxPointsDelta: number | null;
  /** Missing effects always denies external sinks, even when a capability is selected. */
  effects?: EffectGrants;
  automationDelegation?: AutomationDelegation;
}
export const viewerGrant = (): Grant => ({ capabilities: ['live.read'], resources: {}, settingsFields: {}, actions: {}, maxPointsDelta: null });
export interface RemoteLimits {
  connections: number; connectionsPerPrincipal: number; frameBytes: number; paramsBytes: number;
  pendingCalls: number; requestsPerSecond: number; requestBurst: number; commandIdsPerPrincipal: number;
}
export interface Configuration {
  enabled: boolean; bindAddress: string; advertisedOrigin: string | null; allowedOrigins: string[];
  tlsCertificate: string | null; tlsPrivateKey: string | null; tlsProxy: boolean; limits: RemoteLimits;
}
export interface CollaborationStatus {
  hostId: string; hostInstanceId: string; configuration: Configuration; configurationRevision: string;
  activeSessions: number; pendingRequests: number;
  gateway: { available: boolean; running: boolean; connections: number; configurationRevision: string | null;
    address: string | null; lastError: string | null; certificateFingerprint?: string | null; sessionConnections?: Record<string, number> };
}
export interface Invitation { inviteId: string; grant: Grant; expiresAt: number; state: string }
export interface InvitationSecret { inviteId: string; secret: string; expiresAt: number }
export type ClientKind = 'desktop' | 'browser' | 'agent';
export interface PairingRequest { requestId: string; clientLabel: string; clientKind: ClientKind; grant: Grant; state: string; expiresAt: number }
export interface Principal { principalId: string; clientLabel: string; clientKind: ClientKind; grant: Grant; grantRevision: string; revoked: boolean }
export interface Session { sessionId: string; principalId: string; clientLabel?: string; clientKind: ClientKind; createdAt: number; lastActivityAt: number; expiresAt: number; revoked: boolean }
export interface AuditEntry { cursor: number; timestamp: number; principalId: string; action: string; resourceId: string; outcome: string; details?: unknown }
export type AdminTab = 'requests' | 'principals' | 'sessions' | 'invitations' | 'audit';
export type PageRows = { requests: PairingRequest[]; principals: Principal[]; sessions: Session[]; invitations: Invitation[]; audit: AuditEntry[] };
