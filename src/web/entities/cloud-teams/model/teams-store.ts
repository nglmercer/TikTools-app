import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useCloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createActivityCounter, createRequestScope, type RequestTicket } from '@/shared/lib/request-scope.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
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
  type HostLinkStatus,
  type CommandOutcome,
  type RedeemResult,
  type TeamSession,
} from '../api/teams-api.ts';

/**
 * Owner view of remote-control teams. Each team's detail (invitations, requests,
 * grants, sessions) is replaced as a unit, and a stale reply never overwrites a
 * newer selection: closing or changing the team, a new account and closing the panel
 * each end the answers still on their way. An invitation token lives in memory until it
 * is dismissed, and one that arrives for a team no longer selected is never published
 * (the invitation was still issued and can be revoked from that team).
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
  const link = ref<HostLinkStatus>({ state: 'stopped', hostId: null, lastError: null });
  const lastRedeem = ref<RedeemResult | null>(null);
  const lastCommand = ref<CommandOutcome | null>(null);
  const loading = ref(false);
  const activity = createActivityCounter();
  const working = activity.active;
  const error = ref<string | null>(null);
  const { detail: errorDetail, capture, clear: clearDetail } = useCloudFailure();
  const listScope = createRequestScope();
  const detailScope = createRequestScope();
  /** Ends with the selected team: invitations and role changes belong to the team they started for. */
  const selectionScope = createRequestScope();
  /** Ends with the account or the panel: messages and results of operations belong to the view that asked. */
  const viewScope = createRequestScope();
  // Set while a panel is showing this account's teams, so a new context reloads them.
  let shown = false;

  function clearError(): void {
    error.value = null;
    clearDetail();
  }

  /** A new account, session or host starts empty: no earlier team, token, error or late reply survives. */
  useCloudContext(() => {
    const reload = shown;
    reset();
    hosts.value = [];
    devices.value = [];
    lastRedeem.value = null;
    lastCommand.value = null;
    clearError();
    if (reload) void refresh();
  });

  const available = computed(() => capabilities.supports('teams.hosts.list'));
  const selected = computed(() => hosts.value.find((host) => host.id === selectedId.value) ?? null);
  const teamDevices = computed(() => devices.value.filter((device) => device.hostId === selectedId.value));

  /** Runs one operation. Overlapping or nested operations each count; only the view that started one hears its result. */
  async function run(action: (view: RequestTicket) => Promise<void>): Promise<boolean> {
    const view = viewScope.current();
    const leave = activity.enter();
    clearError();
    try {
      await action(view);
      return true;
    } catch (failure) {
      if (view.isCurrent()) error.value = capture(failure);
      return false;
    } finally {
      leave();
    }
  }

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    shown = true;
    const ticket = listScope.begin();
    loading.value = true;
    try {
      const [nextHosts, nextDevices] = await Promise.all([api.hosts(), api.devices()]);
      if (!ticket.isCurrent()) return;
      hosts.value = nextHosts;
      devices.value = nextDevices;
      clearError();
      const nextLink = await api.hostLink().catch(() => null);
      if (!ticket.isCurrent()) return;
      if (nextLink) link.value = nextLink;
      if (selectedId.value && !nextHosts.some((host) => host.id === selectedId.value)) closeTeam();
    } catch (failure) {
      if (ticket.isCurrent()) error.value = capture(failure);
    } finally {
      if (ticket.isCurrent()) loading.value = false;
    }
  }

  /** Loads everything a team owner needs for one team. Replies for other teams are dropped. */
  async function loadDetail(hostId: string): Promise<void> {
    const ticket = detailScope.begin();
    const [nextInvitations, nextRequests, nextGrants, nextSessions] = await Promise.all([
      api.invitations(hostId),
      api.requests(hostId),
      api.grants(hostId),
      api.sessions(hostId),
    ]);
    if (!ticket.isCurrent() || selectedId.value !== hostId) return;
    invitations.value = nextInvitations;
    requests.value = nextRequests;
    grants.value = nextGrants;
    sessions.value = nextSessions;
  }

  async function select(hostId: string): Promise<void> {
    selectionScope.invalidate();
    selectedId.value = hostId;
    issued.value = null;
    await run(() => loadDetail(hostId));
  }

  function closeTeam(): void {
    selectionScope.invalidate();
    detailScope.invalidate();
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
    return run(async (view) => {
      const created = await api.createHost(name.trim());
      if (!view.isCurrent()) return;
      hosts.value = [...hosts.value, created];
      await select(created.id);
    });
  }

  function renameTeam(name: string): Promise<boolean> {
    const host = selected.value;
    if (!host) return Promise.resolve(false);
    return run(async (view) => {
      const updated = await api.updateHost(host.id, { name: name.trim(), expectedRevision: host.revision });
      if (view.isCurrent()) hosts.value = hosts.value.map((item) => (item.id === updated.id ? updated : item));
    });
  }

  function enroll(kind: 'controller' | 'host', label: string): Promise<boolean> {
    const hostId = kind === 'host' ? selectedId.value ?? undefined : undefined;
    return run(async (view) => {
      await api.enroll(kind, label.trim(), hostId);
      const next = await api.devices();
      if (view.isCurrent()) devices.value = next;
    });
  }

  function revokeDevice(deviceId: string): Promise<boolean> {
    return run(async (view) => {
      await api.revokeDevice(deviceId);
      const next = await api.devices();
      if (!view.isCurrent()) return;
      devices.value = next;
      await reloadDetail();
    });
  }

  function createInvitation(requestedRole: TeamRole, ttlSeconds?: number): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    const selection = selectionScope.current();
    return run(async (view) => {
      const created = await api.createInvitation(hostId, requestedRole, ttlSeconds);
      // The token is only shown while the team it was issued for is still the open one.
      if (!view.isCurrent() || !selection.isCurrent()) return;
      issued.value = created;
      const next = await api.invitations(hostId);
      if (selection.isCurrent()) invitations.value = next;
    });
  }

  function dismissInvitation(): void {
    issued.value = null;
  }

  function revokeInvitation(invitationId: string): Promise<boolean> {
    const hostId = selectedId.value;
    if (!hostId) return Promise.resolve(false);
    const selection = selectionScope.current();
    return run(async () => {
      await api.revokeInvitation(hostId, invitationId);
      const next = await api.invitations(hostId);
      if (selection.isCurrent()) invitations.value = next;
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

  /** Joins a team with an invitation token. Redeeming creates a request; the owner decides. */
  function redeem(token: string): Promise<boolean> {
    return run(async (view) => {
      const redeemed = await api.redeem(token.trim());
      if (view.isCurrent()) lastRedeem.value = redeemed;
    });
  }

  /**
   * Sends one command to a team as this PC's viewer. The outcome is shown as the host
   * reported it; a missing result stays `unknown` and is never reported as success.
   */
  function runCommand(hostId: string, operation: string): Promise<boolean> {
    return run(async (view) => {
      const outcome = await api.runCommand(hostId.trim(), operation);
      if (view.isCurrent()) lastCommand.value = outcome;
    });
  }

  /** Starts or stops this PC's link for the selected team. The host must be enrolled for that team. */
  function setHostLink(start: boolean): Promise<boolean> {
    return run(async (view) => {
      const next = start ? await api.startHostLink() : await api.stopHostLink();
      if (view.isCurrent()) link.value = next;
    });
  }

  /**
   * Drops the token and open team detail, for example when the panel unmounts. Replies
   * still on their way no longer touch the state, and the activity starts again.
   */
  function reset(): void {
    shown = false;
    listScope.invalidate();
    viewScope.invalidate();
    activity.reset();
    loading.value = false;
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
    link,
    lastRedeem,
    lastCommand,
    runCommand,
    redeem,
    setHostLink,
    loading,
    working,
    error,
    errorDetail,
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
