import { afterEach, describe, expect, it, vi } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { collaborationStatus, emptyCollaborationPage } from '@/shared/lib/testing/collaboration.ts';
import { viewerGrant } from '../api/types.ts';
import { createCollaborationOwner } from './owner.ts';

const owned: ReturnType<typeof createCollaborationOwner>[] = [];
afterEach(() => { owned.splice(0).forEach((m) => m.stop()); vi.useRealTimers(); });
const copy = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
describe('owner collaboration lifecycle', () => {
  it('loads a session principal by ID without searching a paginated list and rejects a mismatched reply', async () => {
    let returnedId = 'requested-principal';
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'collaboration.status') return collaborationStatus();
      if (method === 'collaboration.principals.get') return { principalId: returnedId, grantRevision: '9', grant: viewerGrant() };
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    expect((await model.getPrincipal('requested-principal'))?.grantRevision).toBe('9');
    expect(calls.find((c) => c.method === 'collaboration.principals.get')?.params).toEqual({ id: 'requested-principal' });
    expect(calls.some((c) => c.method === 'collaboration.principals.list')).toBe(false);
    returnedId = 'another-principal';
    expect(await model.getPrincipal('requested-principal')).toBeNull();
    expect(model.errorCode.value).toBe('protocol');
  });
  it('continues short audit pages until the host returns an empty page', async () => {
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'collaboration.status') return collaborationStatus();
      if (method === 'collaboration.audit.list') return params?.['after'] === 0 ? { entries: [{ cursor: 42, action: 'large-detail' }], nextAfter: 42 } : { entries: [], nextAfter: 42 };
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start(); await model.selectTab('audit');
    expect(model.next.value).toBe(42); await model.nextPage(); expect(model.next.value).toBeNull();
    expect(calls.filter((c) => c.method === 'collaboration.audit.list').map((c) => c.params?.['after'])).toEqual([0, 42]);
  });
  it('keeps a one-use secret only in the mounted owner scope and clears it on expiry', async () => {
    vi.useFakeTimers(); const status = collaborationStatus();
    const issued = { inviteId: 'invite', secret: 'private-secret-marker', expiresAt: Date.now() + 60000 };
    const { client, calls } = createFakeControlClient((method) => method === 'collaboration.status' ? copy(status) : method === 'collaboration.invites.create' ? issued : emptyCollaborationPage(method));
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    await model.createInvitation(viewerGrant());
    expect(model.joinLink.value).toContain('/remote/v1/join#inviteId=invite&secret=private-secret-marker');
    expect(JSON.stringify(model.pages.value)).not.toContain(issued.secret);
    expect(calls.every((c) => c.method.startsWith('collaboration.'))).toBe(true);
    vi.setSystemTime(issued.expiresAt); await vi.advanceTimersByTimeAsync(2000);
    expect(model.secret.value).toBeNull(); expect(model.joinLink.value).toBe('');
    model.stop(); expect(vi.getTimerCount()).toBe(0);
  });
  it('does not retry an unconfirmed mutation or allow another until explicitly reviewed', async () => {
    const status = collaborationStatus(); let attempts = 0;
    const { client } = createFakeControlClient((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.invites.create') { attempts += 1; throw new ControlCallError('transport', 'Lost acknowledgement'); }
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    await model.createInvitation(viewerGrant());
    expect(model.uncertain.value).toBe(true); expect(model.secret.value).toBeNull();
    await model.refresh(); await model.createInvitation(viewerGrant());
    expect(attempts).toBe(1); expect(model.canInvite.value).toBe(false);
    model.uncertain.value = false; await model.createInvitation(viewerGrant()); expect(attempts).toBe(2);
  });
  it('ignores late invitation results and stops reads when the panel has closed', async () => {
    let release: ((v: unknown) => void) | undefined;
    const { client, calls } = createFakeControlClient((method) => method === 'collaboration.status' ? collaborationStatus() : method === 'collaboration.invites.create' ? new Promise((resolve) => { release = resolve; }) : emptyCollaborationPage(method));
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    const create = model.createInvitation(viewerGrant()); await Promise.resolve();
    model.stop(); const count = calls.length;
    release!({ inviteId: 'late', secret: 'must-not-resurface', expiresAt: Date.now() + 60000 }); await create;
    expect(model.secret.value).toBeNull(); expect(calls).toHaveLength(count);
  });
  it('uses bounded cursor pages and discards a previous tab response', async () => {
    let release: ((v: unknown) => void) | undefined;
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'collaboration.status') return collaborationStatus();
      if (method === 'collaboration.requests.list') {
        if (params?.['after'] === 'next-cursor') return new Promise((resolve) => { release = resolve; });
        return { requests: [], nextAfter: 'next-cursor' };
      }
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    const next = model.nextPage(); await Promise.resolve(); await model.selectTab('audit');
    release!({ requests: [{ requestId: 'old-page' }], nextAfter: null }); await next;
    expect(model.tab.value).toBe('audit'); expect(model.pages.value.requests).toEqual([]);
    expect(model.next.value).toBeNull();
    expect(calls.some((c) => c.params?.['after'] === 'next-cursor' && c.params['limit'] === 50)).toBe(true);
  });
  it('passes the original configuration and grant preconditions without overwriting drafts on conflict', async () => {
    const status = collaborationStatus();
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.configure' || method === 'collaboration.grants.set') { status.configurationRevision = '2'; throw new ControlCallError('revision_conflict', 'Changed'); }
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    const draft = copy(status.configuration); draft.bindAddress = '127.0.0.1:8999';
    expect(await model.configure(draft, '1')).toBe(false);
    expect(model.status.value?.configurationRevision).toBe('2'); expect(draft.bindAddress).toBe('127.0.0.1:8999');
    expect(await model.setGrant('principal', '7', viewerGrant())).toBe(false);
    expect(calls.find((c) => c.method === 'collaboration.configure')?.params?.['expectedRevision']).toBe('1');
    expect(calls.find((c) => c.method === 'collaboration.grants.set')?.params?.['expectedRevision']).toBe('7');
    expect(model.uncertain.value).toBe(false);
  });
  it('refuses host identity changes and never reuses a cached owner grant on another host', async () => {
    const status = collaborationStatus();
    const originalHost = status.hostId;
    const { client, calls } = createFakeControlClient((method) => method === 'collaboration.status' ? copy(status) : emptyCollaborationPage(method));
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    status.hostId = 'other-host'; await model.refreshStatus();
    expect(model.connected.value).toBe(false); expect(model.errorCode.value).toBe('host_changed');
    status.hostId = originalHost; await model.refresh();
    expect(model.connected.value).toBe(false); expect(model.errorCode.value).toBe('host_changed');
    await model.act('principals.revoke', 'old-principal');
    expect(calls.some((c) => c.method === 'collaboration.principals.revoke')).toBe(false);
  });
  it('fences pending pages and a private invitation when the owner host changes', async () => {
    const status = collaborationStatus();
    let pageResult: ((value: unknown) => void) | undefined;
    let invitationResult: ((value: unknown) => void) | undefined;
    let hold = false;
    const { client } = createFakeControlClient((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.invites.create') return new Promise((resolve) => { invitationResult = resolve; });
      if (hold && method === 'collaboration.requests.list') return new Promise((resolve) => { pageResult = resolve; });
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); await model.start();
    hold = true;
    const invitation = model.createInvitation(viewerGrant()); const page = model.first(); await Promise.resolve();
    status.hostId = 'other-host'; await model.refreshStatus();
    invitationResult!({ inviteId: 'late', secret: 'must-not-resurface', expiresAt: Date.now() + 60000 });
    pageResult!({ requests: [{ requestId: 'late-request' }], nextAfter: null });
    await Promise.all([invitation, page]);
    expect(model.secret.value).toBeNull(); expect(model.pages.value.requests).toEqual([]);
    expect(model.connected.value).toBe(false); expect(model.loading.value).toBe(false);
  });
  it('reads every list for the access matrix, following cursors, and keeps refreshing it instead of one paged tab', async () => {
    const principal = (id: string) => ({ principalId: id, clientLabel: id, clientKind: 'browser', grant: viewerGrant(), grantRevision: '1', revoked: false });
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'collaboration.status') return collaborationStatus();
      if (method === 'collaboration.principals.list') return params?.['after'] === null ? { principals: [principal('a')], nextAfter: 'next' } : { principals: [principal('b')], nextAfter: null };
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); model.watchPeople(); await model.start();
    expect(model.people.value.principals.map((p) => p.principalId)).toEqual(['a', 'b']);
    expect(calls.filter((c) => c.method === 'collaboration.principals.list').map((c) => c.params?.['after'])).toEqual([null, 'next']);
    for (const kind of ['requests', 'sessions']) expect(calls.some((c) => c.method === `collaboration.${kind}.list`)).toBe(true);
  });
  it('rejects a cursor that does not advance while reading the matrix', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'collaboration.status') return collaborationStatus();
      if (method === 'collaboration.principals.list') return { principals: [], nextAfter: 'same' };
      return emptyCollaborationPage(method);
    });
    const model = createCollaborationOwner(client); owned.push(model); model.watchPeople(); await model.start();
    expect(model.errorCode.value).toBe('protocol');
  });
});
