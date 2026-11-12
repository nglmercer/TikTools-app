import { computed, ref } from 'vue';
import { ControlCallError, errorMessage, type ControlClient, type RpcParams } from '@/shared/api/control-client.ts';
import type { AdminTab, CollaborationStatus, Configuration, Grant, InvitationSecret, PageRows, PairingRequest, Principal, Session } from '../api/types.ts';

const pageMethods = { requests: 'requests', principals: 'principals', sessions: 'sessions', invitations: 'invites', audit: 'audit' } as const;
const pageKeys = { requests: 'requests', principals: 'principals', sessions: 'sessions', invitations: 'invitations', audit: 'entries' } as const;
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

/** Owned by the mounted owner panel. Secrets never enter a global store. */
export function createCollaborationOwner(client: ControlClient) {
  const status = ref<CollaborationStatus | null>(null);
  const tab = ref<AdminTab>('requests');
  const pages = ref<PageRows>({ requests: [], principals: [], sessions: [], invitations: [], audit: [] });
  const after = ref<string | number | null>(null);
  const next = ref<string | number | null>(null);
  const previous = ref<Array<string | number | null>>([]);
  const loading = ref(false);
  const working = ref(false);
  const error = ref('');
  const errorCode = ref('');
  const uncertain = ref(false);
  const connected = ref(false);
  const checkedAt = ref<number | null>(null);
  const secret = ref<InvitationSecret | null>(null);
  /** Everyone at once (requests, identities, sessions) for the access matrix; independent of the paged tabs. */
  const people = ref<{ requests: PairingRequest[]; principals: Principal[]; sessions: Session[] }>({ requests: [], principals: [], sessions: [] });
  const peopleLoading = ref(false);
  const now = ref(Date.now());
  const live = new AbortController();
  let disposed = false;
  let hostChanged = false;
  let pageGeneration = 0;
  let statusGeneration = 0;
  let pageAbort: AbortController | undefined;
  let timer: ReturnType<typeof setInterval> | undefined;
  let polling = false;
  let trackPeople = false;
  let peopleGeneration = 0;
  let peopleAbort: AbortController | undefined;
  const canInvite = computed(() => connected.value && !!status.value?.configuration.enabled && status.value.gateway.running &&
    status.value.configurationRevision === status.value.gateway.configurationRevision && !working.value && !uncertain.value);
  const joinLink = computed(() => {
    const origin = status.value?.configuration.advertisedOrigin;
    const invitation = secret.value;
    const s = status.value;
    if (!connected.value || !s?.configuration.enabled || !s.gateway.running || s.gateway.configurationRevision !== s.configurationRevision || !origin || !invitation || invitation.expiresAt <= now.value) return '';
    return `${origin}/remote/v1/join#${new URLSearchParams({ inviteId: invitation.inviteId, secret: invitation.secret })}`;
  });
  function report(failure: unknown): void {
    error.value = errorMessage(failure);
    errorCode.value = failure instanceof ControlCallError ? failure.code : 'error';
  }
  function invalidateHost(): void {
    hostChanged = true; secret.value = null; connected.value = false;
    pageGeneration += 1; pageAbort?.abort(); loading.value = false;
    pages.value = { requests: [], principals: [], sessions: [], invitations: [], audit: [] };
    peopleGeneration += 1; peopleAbort?.abort(); peopleLoading.value = false;
    people.value = { requests: [], principals: [], sessions: [] };
    after.value = null; next.value = null; previous.value = [];
    report(new ControlCallError('host_changed', 'Host identity changed; reopen the collaboration panel'));
  }
  async function refreshStatus(): Promise<void> {
    if (disposed || hostChanged) return;
    const generation = ++statusGeneration;
    try {
      const result = await client.call<CollaborationStatus>('collaboration.status', {}, { signal: live.signal });
      if (disposed || generation !== statusGeneration) return;
      if (status.value && status.value.hostId !== result.hostId) {
        invalidateHost();
        return;
      }
      status.value = result;
      connected.value = true; checkedAt.value = Date.now();
      if (!result.configuration.enabled) secret.value = null;
    } catch (failure) { if (!disposed && generation === statusGeneration) { connected.value = false; report(failure); } }
  }
  async function loadPage(cursor: string | number | null = after.value): Promise<void> {
    if (disposed || hostChanged) return;
    const selected = tab.value;
    const generation = ++pageGeneration;
    pageAbort?.abort(); pageAbort = new AbortController();
    const abort = pageAbort;
    const onDispose = () => abort.abort(); live.signal.addEventListener('abort', onDispose, { once: true });
    loading.value = true;
    try {
      const params: RpcParams = selected === 'audit' ? { after: cursor ?? 0, limit: 50 } : { after: cursor, limit: 50 };
      const result = await client.call<Record<string, unknown>>(`collaboration.${pageMethods[selected]}.list`, params, { signal: abort.signal });
      if (disposed || generation !== pageGeneration) return;
      const rows = result[pageKeys[selected]];
      if (!Array.isArray(rows) || rows.length > 100) throw new ControlCallError('protocol', 'Invalid collaboration page');
      const following = result['nextAfter'];
      if (selected === 'audit') {
        if (typeof following !== 'number' || !Number.isSafeInteger(following) || following < 0) throw new ControlCallError('protocol', 'Unsupported audit cursor');
        // Audit pages may stop early at their byte budget. A final empty page
        // is preferable to silently omitting remaining records.
        next.value = rows.length > 0 && following !== (cursor ?? 0) ? following : null;
      } else {
        if (following !== null && typeof following !== 'string') throw new ControlCallError('protocol', 'Invalid collaboration cursor');
        if (following !== null && following === cursor) throw new ControlCallError('protocol', 'Collaboration cursor did not advance');
        next.value = following;
      }
      Object.assign(pages.value, { [selected]: rows });
      after.value = cursor;
    } catch (failure) { if (!disposed && generation === pageGeneration && !abort.signal.aborted) report(failure); }
    finally { live.signal.removeEventListener('abort', onDispose); if (generation === pageGeneration) loading.value = false; }
  }
  /** Reads every page of one list, bounded so a hostile or huge host cannot grow the panel without limit. */
  async function readAll<T>(kind: 'requests' | 'principals' | 'sessions', signal: AbortSignal): Promise<T[]> {
    const rows: T[] = [];
    let cursor: string | null = null;
    for (let page = 0; page < 4; page += 1) {
      const result: Record<string, unknown> = await client.call<Record<string, unknown>>(`collaboration.${pageMethods[kind]}.list`, { after: cursor, limit: 50 }, { signal });
      const batch = result[pageKeys[kind]];
      if (!Array.isArray(batch) || batch.length > 100) throw new ControlCallError('protocol', 'Invalid collaboration page');
      rows.push(...(batch as T[]));
      const following = result['nextAfter'];
      if (following !== null && typeof following !== 'string') throw new ControlCallError('protocol', 'Invalid collaboration cursor');
      if (following === null) break;
      if (following === cursor) throw new ControlCallError('protocol', 'Collaboration cursor did not advance');
      cursor = following;
    }
    return rows;
  }
  async function loadPeople(): Promise<void> {
    if (disposed || hostChanged) return;
    const generation = ++peopleGeneration;
    peopleAbort?.abort(); peopleAbort = new AbortController();
    const abort = peopleAbort;
    const onDispose = () => abort.abort(); live.signal.addEventListener('abort', onDispose, { once: true });
    peopleLoading.value = true;
    try {
      const [requests, principals, sessions] = await Promise.all([
        readAll<PairingRequest>('requests', abort.signal), readAll<Principal>('principals', abort.signal), readAll<Session>('sessions', abort.signal),
      ]);
      if (disposed || generation !== peopleGeneration) return;
      people.value = { requests, principals, sessions };
    } catch (failure) { if (!disposed && generation === peopleGeneration && !abort.signal.aborted) report(failure); }
    finally { live.signal.removeEventListener('abort', onDispose); if (generation === peopleGeneration) peopleLoading.value = false; }
  }
  /** The matrix replaces the paged tabs: from now on refreshes read the whole access picture instead. */
  function watchPeople(): void { trackPeople = true; }
  const reload = (): Promise<void> => trackPeople ? loadPeople() : loadPage();
  async function selectTab(selected: AdminTab): Promise<void> {
    tab.value = selected; after.value = null; next.value = null; previous.value = [];
    await loadPage(null);
  }
  async function first(): Promise<void> { previous.value = []; await loadPage(null); }
  async function nextPage(): Promise<void> {
    if (next.value === null || loading.value) return;
    const cursor = next.value;
    previous.value = [...previous.value.slice(-127), after.value]; await loadPage(cursor);
  }
  async function previousPage(): Promise<void> { if (!previous.value.length || loading.value) return; await loadPage(previous.value.pop() ?? null); }
  async function refresh(): Promise<void> { await Promise.all([refreshStatus(), reload()]); }
  async function mutate<T>(method: string, params: RpcParams): Promise<T | null> {
    if (working.value || disposed || uncertain.value || !connected.value) return null;
    working.value = true; error.value = ''; errorCode.value = '';
    try {
      // Sent once. Cancelling a page cannot undo an already accepted operation.
      const result = await client.call<T>(method, params, { signal: live.signal });
      if (disposed || hostChanged) return null;
      return result;
    } catch (failure) {
      if (!disposed && !hostChanged) {
        report(failure);
        uncertain.value = ['transport', 'timeout', 'cancelled', 'protocol'].includes(errorCode.value);
        await refreshStatus();
      }
      return null;
    } finally { working.value = false; }
  }
  async function configure(configuration: Configuration, expectedRevision: string): Promise<boolean> {
    const result = await mutate<CollaborationStatus>('collaboration.configure', { ...clone(configuration), expectedRevision });
    if (!result) return false;
    if (status.value && result.hostId !== status.value.hostId) { invalidateHost(); return false; }
    status.value = result; if (!result.configuration.enabled) secret.value = null;
    await reload(); return true;
  }
  async function createInvitation(grant: Grant): Promise<void> {
    if (!canInvite.value) return;
    secret.value = null;
    const result = await mutate<InvitationSecret>('collaboration.invites.create', { grant: clone(grant) });
    if (result && !disposed) { now.value = Date.now(); secret.value = result.expiresAt > now.value ? result : null; }
    await refresh();
  }
  async function act(action: 'requests.approve' | 'requests.reject' | 'sessions.revoke' | 'principals.revoke' | 'invites.revoke', id: string): Promise<boolean> {
    const result = await mutate(`collaboration.${action}`, { id });
    if (result && action === 'invites.revoke' && id === secret.value?.inviteId) secret.value = null;
    await refresh(); return result !== null;
  }
  async function setGrant(principalId: string, expectedRevision: string, grant: Grant): Promise<boolean> {
    const result = await mutate('collaboration.grants.set', { principalId, expectedRevision, grant: clone(grant) });
    await refresh(); return result !== null;
  }
  async function getPrincipal(id: string): Promise<Principal | null> {
    if (disposed || !connected.value) return null;
    try {
      const principal = await client.call<Principal>('collaboration.principals.get', { id }, { signal: live.signal });
      if (disposed || !connected.value) return null;
      if (principal.principalId !== id) throw new ControlCallError('protocol', 'Principal reply did not match its request');
      return principal;
    } catch (failure) { if (!disposed && !hostChanged) report(failure); return null; }
  }
  async function start(): Promise<void> {
    await refresh();
    if (disposed) return;
    timer = setInterval(() => {
      now.value = Date.now();
      if (secret.value && secret.value.expiresAt <= now.value) secret.value = null;
      if (polling || working.value) return;
      polling = true;
      void Promise.all([refreshStatus(), ...(trackPeople ? [loadPeople()] : tab.value === 'requests' ? [loadPage()] : [])]).finally(() => { polling = false; });
    }, 2000);
  }
  function stop(): void {
    disposed = true; live.abort(); pageAbort?.abort(); peopleAbort?.abort(); if (timer) clearInterval(timer);
    secret.value = null; status.value = null; connected.value = false;
    pages.value = { requests: [], principals: [], sessions: [], invitations: [], audit: [] };
    people.value = { requests: [], principals: [], sessions: [] };
  }
  return { status, tab, pages, people, peopleLoading, watchPeople, after, next, previous, loading, working, error, errorCode, uncertain, connected, checkedAt, secret, now, canInvite, joinLink,
    start, stop, refresh, refreshStatus, selectTab, first, nextPage, previousPage, configure, createInvitation, act, setGrant, getPrincipal };
}
