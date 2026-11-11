import type { ControlClient } from '@/shared/api/control-client.ts';

export const PRESET_ROLES = ['observer', 'moderator', 'automationEditor', 'custom'] as const;
export type TeamRole = (typeof PRESET_ROLES)[number];

export interface TeamHost {
  readonly id: string;
  readonly name: string;
  readonly status: string;
  readonly protocolVersion: number;
  readonly revision: number;
  readonly online: boolean;
}

export interface TeamDevice {
  readonly id: string;
  readonly label: string;
  readonly kind: string;
  readonly hostId: string | null;
  readonly status: string;
  readonly createdAt: string;
}

export interface TeamInvitation {
  readonly id: string;
  readonly requestedRole: string;
  readonly status: string;
  readonly expiresAt: string;
  readonly redeemedAt: string | null;
}

/** The only shape that carries a token. Shown once, held in memory, then dropped. */
export interface IssuedInvitation {
  readonly id: string;
  readonly requestedRole: string;
  readonly expiresAt: string;
  readonly token: string;
}

export interface TeamRequest {
  readonly id: string;
  readonly requestedRole: string;
  readonly status: string;
  readonly deviceLabel: string;
  readonly createdAt: string;
}

export interface TeamGrant {
  readonly id: string;
  readonly deviceId: string;
  readonly deviceLabel: string;
  readonly role: string;
  readonly status: string;
  readonly revision: number;
}

export interface TeamSession {
  readonly id: string;
  readonly grantId: string;
  readonly deviceId: string;
  readonly grantRevision: number;
  readonly openedAt: string;
}

export interface RedeemResult {
  readonly requestId: string;
  readonly hostId: string;
  readonly status: string;
  readonly requestedRole: string;
}

export function parseRedeemResult(value: unknown): RedeemResult {
  const result = record(value);
  return {
    requestId: id(result.requestId),
    hostId: id(result.hostId),
    status: text(result.status),
    requestedRole: text(result.requestedRole),
  };
}

export interface CommandOutcome {
  readonly commandId: string;
  /** `completed`, `failed`, `cancelled`, `expired` or `unknown`. `unknown` means no result arrived. */
  readonly status: string;
  readonly code: string | null;
  readonly result: unknown;
}

export function parseCommandOutcome(value: unknown): CommandOutcome {
  const outcome = record(value);
  return {
    commandId: text(outcome.commandId),
    status: text(outcome.status),
    code: optionalText(outcome.code),
    result: outcome.result ?? null,
  };
}

export interface HostLinkStatus {
  readonly state: 'stopped' | 'connecting' | 'ready' | 'retrying' | string;
  readonly hostId: string | null;
  readonly lastError: string | null;
}

export function parseHostLink(value: unknown): HostLinkStatus {
  const link = record(value);
  return {
    state: text(link.state),
    hostId: optionalText(link.hostId),
    lastError: optionalText(link.lastError),
  };
}

export class InvalidTeamResponse extends Error {
  constructor() {
    super('invalid team response');
    this.name = 'InvalidTeamResponse';
  }
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
type Json = Record<string, unknown>;

function record(value: unknown): Json {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new InvalidTeamResponse();
  return value as Json;
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new InvalidTeamResponse();
  return value;
}
function optionalText(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  return text(value);
}
function number(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) throw new InvalidTeamResponse();
  return value;
}
function items(value: unknown): unknown[] {
  const page = record(value);
  if (!Array.isArray(page.items)) throw new InvalidTeamResponse();
  return page.items;
}
function id(value: unknown): string {
  const raw = text(value);
  if (!UUID.test(raw)) throw new InvalidTeamResponse();
  return raw;
}

export function parseHost(value: unknown): TeamHost {
  const host = record(value);
  return {
    id: id(host.id),
    name: text(host.name),
    status: text(host.status),
    protocolVersion: number(host.protocolVersion),
    revision: number(host.revision),
    online: host.online === true,
  };
}
export const parseHosts = (value: unknown): TeamHost[] => items(value).map(parseHost);

export function parseDevice(value: unknown): TeamDevice {
  const device = record(value);
  return {
    id: id(device.id),
    label: text(device.label),
    kind: text(device.kind),
    hostId: optionalText(device.hostId),
    status: text(device.status),
    createdAt: typeof device.createdAt === 'string' ? device.createdAt : '',
  };
}
export const parseDevices = (value: unknown): TeamDevice[] => items(value).map(parseDevice);

export const parseInvitations = (value: unknown): TeamInvitation[] =>
  items(value).map((entry) => {
    const invitation = record(entry);
    return {
      id: id(invitation.id),
      requestedRole: text(invitation.requestedRole),
      status: text(invitation.status),
      expiresAt: text(invitation.expiresAt),
      redeemedAt: optionalText(invitation.redeemedAt),
    };
  });

export function parseIssuedInvitation(value: unknown): IssuedInvitation {
  const invitation = record(value);
  const token = text(invitation.token);
  if (token.length < 32) throw new InvalidTeamResponse();
  return {
    id: id(invitation.id),
    requestedRole: text(invitation.requestedRole),
    expiresAt: text(invitation.expiresAt),
    token,
  };
}

export const parseRequests = (value: unknown): TeamRequest[] =>
  items(value).map((entry) => {
    const request = record(entry);
    return {
      id: id(request.id),
      requestedRole: text(request.requestedRole),
      status: text(request.status),
      deviceLabel: text(request.deviceLabel),
      createdAt: text(request.createdAt),
    };
  });

export const parseGrants = (value: unknown): TeamGrant[] =>
  items(value).map((entry) => {
    const grant = record(entry);
    return {
      id: id(grant.id),
      deviceId: id(grant.deviceId),
      deviceLabel: text(grant.deviceLabel),
      role: text(grant.role),
      status: text(grant.status),
      revision: number(grant.revision),
    };
  });

export const parseSessions = (value: unknown): TeamSession[] =>
  items(value).map((entry) => {
    const session = record(entry);
    return {
      id: id(session.id),
      grantId: id(session.grantId),
      deviceId: id(session.deviceId),
      grantRevision: number(session.grantRevision),
      openedAt: text(session.openedAt),
    };
  });

export type Call = (method: string, params?: Record<string, unknown>) => Promise<unknown>;

/** Thin wrappers over the host methods. Each reply is parsed before it reaches the store. */
export function teamApi(client: ControlClient): {
  hosts: () => Promise<TeamHost[]>;
  createHost: (name: string) => Promise<TeamHost>;
  updateHost: (hostId: string, patch: { name?: string; status?: string; expectedRevision: number }) => Promise<TeamHost>;
  devices: () => Promise<TeamDevice[]>;
  enroll: (kind: 'controller' | 'host', label: string, hostId?: string) => Promise<TeamDevice>;
  revokeDevice: (deviceId: string) => Promise<void>;
  invitations: (hostId: string) => Promise<TeamInvitation[]>;
  createInvitation: (hostId: string, requestedRole: TeamRole, ttlSeconds?: number) => Promise<IssuedInvitation>;
  revokeInvitation: (hostId: string, invitationId: string) => Promise<void>;
  requests: (hostId: string) => Promise<TeamRequest[]>;
  decide: (hostId: string, requestId: string, decision: 'approve' | 'deny', role?: TeamRole) => Promise<void>;
  grants: (hostId: string) => Promise<TeamGrant[]>;
  updateGrantRole: (hostId: string, grantId: string, role: TeamRole, expectedRevision: number) => Promise<void>;
  revokeGrant: (hostId: string, grantId: string, expectedRevision: number) => Promise<void>;
  sessions: (hostId: string) => Promise<TeamSession[]>;
  revokeSession: (hostId: string, sessionId: string) => Promise<void>;
  redeem: (token: string) => Promise<RedeemResult>;
  runCommand: (hostId: string, operation: string) => Promise<CommandOutcome>;
  hostLink: () => Promise<HostLinkStatus>;
  startHostLink: () => Promise<HostLinkStatus>;
  stopHostLink: () => Promise<HostLinkStatus>;
} {
  const call = client.call.bind(client) as Call;
  return {
    hosts: async () => parseHosts(await call('teams.hosts.list', {})),
    createHost: async (name) => parseHost(await call('teams.hosts.create', { name })),
    updateHost: async (hostId, patch) => parseHost(await call('teams.hosts.update', { hostId, ...patch })),
    devices: async () => parseDevices(await call('teams.devices.list', {})),
    enroll: async (kind, label, hostId) =>
      parseDevice(await call('teams.devices.enroll', { kind, label, ...(hostId ? { hostId } : {}) })),
    revokeDevice: async (deviceId) => void (await call('teams.devices.revoke', { deviceId })),
    invitations: async (hostId) => parseInvitations(await call('teams.invitations.list', { hostId })),
    createInvitation: async (hostId, requestedRole, ttlSeconds) =>
      parseIssuedInvitation(await call('teams.invitations.create', {
        hostId, requestedRole, ...(ttlSeconds ? { ttlSeconds } : {}),
      })),
    revokeInvitation: async (hostId, invitationId) => void (await call('teams.invitations.revoke', { hostId, invitationId })),
    requests: async (hostId) => parseRequests(await call('teams.requests.list', { hostId })),
    decide: async (hostId, requestId, decision, role) =>
      void (await call('teams.requests.decide', { hostId, requestId, decision, ...(role ? { role } : {}) })),
    grants: async (hostId) => parseGrants(await call('teams.grants.list', { hostId })),
    updateGrantRole: async (hostId, grantId, role, expectedRevision) =>
      void (await call('teams.grants.update', { hostId, grantId, role, expectedRevision })),
    revokeGrant: async (hostId, grantId, expectedRevision) =>
      void (await call('teams.grants.revoke', { hostId, grantId, expectedRevision })),
    sessions: async (hostId) => parseSessions(await call('teams.sessions.list', { hostId })),
    revokeSession: async (hostId, sessionId) => void (await call('teams.sessions.revoke', { hostId, sessionId })),
    redeem: async (token) => parseRedeemResult(await call('teams.invitations.redeem', { token })),
    runCommand: async (hostId, operation) => parseCommandOutcome(await call('teams.commands.run', { hostId, operation })),
    hostLink: async () => parseHostLink(await call('teams.host.status', {})),
    startHostLink: async () => parseHostLink(await call('teams.host.start', {})),
    stopHostLink: async () => parseHostLink(await call('teams.host.stop', {})),
  };
}
