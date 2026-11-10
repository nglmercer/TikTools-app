import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import {
  teamApi,
  type IssuedInvitation,
  type TeamDevice,
  type TeamGrant,
  type TeamHost,
  type TeamInvitation,
  type TeamRequest,
  type TeamRole,
  type TeamSession,
} from '../api/teams-api.ts';

/**
 * Owner view of remote-control teams. Each team's detail (invitations, requests,
 * grants, sessions) is replaced as a unit, and a stale reply never overwrites a
 * newer selection. An invitation token lives in memory until it is dismissed.
 */
export const useTeamsStore = defineStore('cloud-teams', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const api = teamApi(client);

  const hosts = ref<readonly TeamHost[]>([]);
  const devices = ref<readonly TeamDevice[]>([]);
  const selectedId = ref<string | null>(null);
  const invitations = ref<readonly TeamInvitation[]>([]);
  const requests = ref<readonly TeamRequest[]>([]);
  const grants = ref<readonly TeamGrant[]>([]);
  const sessions = ref<readonly TeamSession[]>([]);
  const issued = ref<IssuedInvitation | null>(null);
  const loading = ref(false);
  const working = ref(false);
  const error = ref<string | null>(null);
  let listGeneration = 0;
  let detailGeneration = 0;

  const available = computed(() => capabilities.supports('teams.hosts.list'));
  const selected = computed(() => hosts.value.find((host) => host.id === selectedId.value) ?? null);
  const teamDevices = computed(() => devices.value.filter((device) => device.hostId === selectedId.value));

  async function run(action: () => Promise<void>): Promise<boolean> {
    working.value = true;
    error.value = null;
    try {
      await action();
      return true;
    } catch (failure) {
      error.value = errorMessage(failure);
      return false;
    } finally {
      working.value = false;
    }
  }

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    const ticket = ++listGeneration;
    loading.value = true;
    try {
      const [nextHosts, nextDevices] = await Promise.all([api.hosts(), api.devices()]);
      if (ticket !== listGeneration) return;
      hosts.value = nextHosts;
      devices.value = nextDevices;
      error.value = null;
      if (selectedId.value && !nextHosts.some((host) => host.id === selectedId.value)) closeTeam();
    } catch (failure) {
      if (ticket === listGeneration) error.value = errorMessage(failure);
    } finally {
      if (ticket === listGeneration) loading.value = false;
    }
  }

  /** Loads everything a team owner needs for one team. Replies for other teams are dropped. */
  async function loadDetail(hostId: string): Promise<void> {
    const ticket = ++detailGeneration;
    const [nextInvitations, nextRequests, nextGrants, nextSessions] = await Promise.all([
      api.invitations(hostId),
      api.requests(hostId),
      api.grants(hostId),
      api.sessions(hostId),
    ]);
    if (ticket !== detailGeneration || selectedId.value !== hostId) return;
    invitations.value = nextInvitations;
    requests.value = nextRequests;
    grants.value = nextGrants;
    sessions.value = nextSessions;
  }

  async function select(hostId: string): Promise<void> {
    selectedId.value = hostId;
    issued.value = null;
    await run(() => loadDetail(hostId));
  }

  function closeTeam(): void {
    detailGeneration += 1;
    selectedId.value = null;
    invitations.value = [];
    requests.value = [];
    grants.value = [];
    sessions.value = [];
    issued.value = null;
  }

  async function reloadDetail(): Promise<void> {
    if (selectedId.value) await loadDetail(selectedId.value);
  }

  function createTeam(name: string): Promise<boolean> {
    return run(async () => {
      const created = await api.createHost(name.trim());
      hosts.value = [...hosts.value, created];
      await select(created.id);
    });
  }

  function renameTeam(name: string): Promise<boolean> {
    const host = selected.value;
    if (!host) return Promise.resolve(false);
    return run(async () => {
      const updated = await api.updateHost(host.id, { name: name.trim(), expectedRevision: host.revision });
      hosts.value = hosts.value.map((item) => (item.id === updated.id ? updated : item));
    });
  }

  function enroll(kind: 'controller' | 'host', label: string): Promise<boolean> {
    const hostId = kind === 'host' ? selectedId.value ?? undefined : undefined;
    return run(async () => {
      await api.enroll(kind, label.trim(), hostId);
      devices.value = await api.devices();
    });
  }

  function revokeDevice(deviceId: string): Promise<boolean> {
    return run(async () => {
      await api.revokeDevice(deviceId);
      devices.value = await api.devices();
      await reloadDetail();
    });
  }

  function createInvitation(requestedRole: TeamRole, ttlSeconds?: number): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      issued.value = await api.createInvitation(hostId, requestedRole, ttlSeconds);
      invitations.value = await api.invitations(hostId);
    });
  }

  function dismissInvitation(): void {
    issued.value = null;
  }

  function revokeInvitation(invitationId: string): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      await api.revokeInvitation(hostId, invitationId);
      invitations.value = await api.invitations(hostId);
    });
  }

  function decide(requestId: string, decision: 'approve' | 'deny', role?: TeamRole): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      await api.decide(hostId, requestId, decision, role);
      await loadDetail(hostId);
    });
  }

  /** Changing a role returns the grant to the host for confirmation and closes its sessions. */
  function changeRole(grant: TeamGrant, role: TeamRole): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      await api.updateGrantRole(hostId, grant.id, role, grant.revision);
      await loadDetail(hostId);
    });
  }

  function revokeGrant(grant: TeamGrant): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      await api.revokeGrant(hostId, grant.id, grant.revision);
      await loadDetail(hostId);
    });
  }

  function revokeSession(sessionId: string): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    return run(async () => {
      await api.revokeSession(hostId, sessionId);
      await loadDetail(hostId);
    });
  }

  /** Drops the token and open team detail, for example when the panel unmounts. */
  function reset(): void {
    closeTeam();
  }

  return {
    hosts,
    devices: teamDevices,
    allDevices: devices,
    selected,
    selectedId,
    invitations,
    requests,
    grants,
    sessions,
    issued,
    loading,
    working,
    error,
    available,
    refresh,
    select,
    createTeam,
    renameTeam,
    enroll,
    revokeDevice,
    createInvitation,
    dismissInvitation,
    revokeInvitation,
    decide,
    changeRole,
    revokeGrant,
    revokeSession,
    reset,
  };
});
