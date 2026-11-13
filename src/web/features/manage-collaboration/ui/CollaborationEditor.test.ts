import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { collaborationStatus, emptyCollaborationPage } from '@/shared/lib/testing/collaboration.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { viewerGrant } from '@/entities/collaboration/api/types.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import CollaborationEditor from './CollaborationEditor.vue';

const { toCanvas } = vi.hoisted(() => ({ toCanvas: vi.fn<(canvas: HTMLCanvasElement, value: string, options: unknown) => Promise<void>>().mockResolvedValue(undefined) }));
vi.mock('qrcode', () => ({ toCanvas }));
enableAutoUnmount(afterEach);
beforeEach(() => { i18n.global.locale.value = 'en'; toCanvas.mockClear(); });
const copy = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
function mountPanel(handler: Parameters<typeof createFakeControlClient>[0]) {
  const fake = createFakeControlClient(handler);
  const wrapper = mountWithPlugins(CollaborationEditor, { attachTo: document.body, global: { provide: { [controlClientKey as symbol]: fake.client } } });
  const button = (text: string) => wrapper.findAll('button').find((node) => node.text() === text)!;
  // The share dialog is also mounted (closed): the open confirmation is the last one with content.
  const dialog = () => wrapper.findAll('dialog').filter((node) => node.text() !== '').at(-1)!;
  const cell = (label: string) => wrapper.find(`button[aria-label="${label}"]`);
  return { ...fake, wrapper, button, cell, dialog };
}
const person = { principalId: 'person-1', clientLabel: 'Moderator PC', clientKind: 'desktop', grant: viewerGrant(), grantRevision: '1', revoked: false };

describe('owner collaboration panel', () => {
  it('uses the native gateway and saves its Cloud configuration without manual HTTPS setup', async () => {
    const status = collaborationStatus();
    status.peerGateway = { ...status.gateway, available: true, running: true, connections: 2 };
    status.gateway = { ...status.gateway, available: false, running: false };
    status.configuration.peer = { relayUrls: [] };
    status.configuration.advertisedOrigin = null;
    status.configuration.tlsCertificate = null;
    status.configuration.tlsPrivateKey = null;
    const { wrapper, calls, button } = mountPanel(method => method === 'collaboration.status' ? copy(status) : method === 'collaboration.configure' ? { ok: true } : emptyCollaborationPage(method));
    await flushPromises();
    expect(wrapper.text()).not.toContain(i18n.global.t('collaboration.unavailable'));
    expect(wrapper.text()).toContain(i18n.global.t('collaboration.connected', { count: 2 }));
    await button('Connection settings').trigger('click');
    expect(wrapper.find('#collaboration-bind').exists()).toBe(false);
    expect(wrapper.find('#collaboration-certificate').exists()).toBe(false);
    expect(wrapper.find('#collaboration-key').exists()).toBe(false);
    await button(i18n.global.t('cloudTeams.title')).trigger('click');
    expect(wrapper.emitted('teams')).toHaveLength(1);
    await wrapper.get('form').trigger('submit'); await flushPromises();
    const saved = calls.find(call => call.method === 'collaboration.configure');
    expect(saved?.params?.['peer']).toEqual({ relayUrls: [] });
    expect(saved?.params?.['enabled']).toBe(true);
    expect(saved?.params?.['advertisedOrigin']).toBeNull();
  });
  it('shows everyone and what they can do on one screen without writing anything', async () => {
    const status = collaborationStatus();
    const { wrapper, calls } = mountPanel((method) => method === 'collaboration.status' ? copy(status) : method === 'collaboration.principals.list' ? { principals: [person], nextAfter: null } : emptyCollaborationPage(method));
    await flushPromises();
    expect(wrapper.get('[role="table"]').text()).toContain('Moderator PC');
    expect(wrapper.find('button[aria-label="Moderator PC, LIVE: View"]').exists()).toBe(true);
    expect(wrapper.find('button[aria-label="Moderator PC, Points: None"]').exists()).toBe(true);
    expect(wrapper.find('[role="tablist"]').exists()).toBe(false);
    expect(calls.some((c) => ['collaboration.configure', 'collaboration.invites.create', 'collaboration.grants.set'].includes(c.method))).toBe(false);
  });

  it('keeps the connection draft while browsing and does not write on navigation', async () => {
    const status = collaborationStatus();
    const { wrapper, calls, button } = mountPanel(method => method === 'collaboration.status' ? copy(status) : emptyCollaborationPage(method));
    await flushPromises();
    expect(wrapper.find('#collaboration-bind').exists()).toBe(false);
    await button('Connection settings').trigger('click');
    await wrapper.get('#collaboration-bind').setValue('127.0.0.1:8999');
    expect(wrapper.text()).toContain('You have unsaved changes.');
    await button('← Access matrix').trigger('click');
    expect(wrapper.find('#collaboration-bind').exists()).toBe(false);
    await button('Connection settings').trigger('click');
    expect((wrapper.get('#collaboration-bind').element as HTMLInputElement).value).toBe('127.0.0.1:8999');
    expect(calls.some(call => call.method === 'collaboration.configure')).toBe(false);
  });

  it('checks the draft, not the file system, and warns about listening on every interface', async () => {
    const status = collaborationStatus();
    const { wrapper, button } = mountPanel(method => method === 'collaboration.status' ? copy(status) : emptyCollaborationPage(method));
    await flushPromises(); await button('Connection settings').trigger('click');
    expect(wrapper.text()).toContain('The public address uses HTTPS');
    expect(wrapper.text()).not.toContain('Listening on all interfaces');
    await wrapper.get('#collaboration-bind').setValue('0.0.0.0:8788');
    expect(wrapper.text()).toContain('Listening on all interfaces');
    await wrapper.get('#collaboration-origin').setValue('');
    expect(wrapper.text()).toContain('Set a public HTTPS address');
  });

  it('creates an invitation from the new row, shares a fragment link and generates the QR only on request', async () => {
    const status = collaborationStatus(); const privateSecret = 'b'.repeat(64);
    const { wrapper, calls, button } = mountPanel((method) => method === 'collaboration.status' ? copy(status) : method === 'collaboration.invites.create' ? { inviteId: 'invite-id', secret: privateSecret, expiresAt: Date.now() + 60000 } : emptyCollaborationPage(method));
    await flushPromises();
    expect(calls.some((c) => c.method === 'collaboration.invites.create')).toBe(false);
    await wrapper.get('button[aria-label="+ New invitation, Points: None"]').trigger('click');
    await wrapper.findAll('[role="radio"]').find((node) => node.text() === 'Manage')!.trigger('click');
    await button('Create private invitation').trigger('click'); await flushPromises();
    const sent = calls.find((c) => c.method === 'collaboration.invites.create')?.params?.['grant'] as { capabilities: string[] };
    expect(sent.capabilities).toEqual(expect.arrayContaining(['live.read', 'points.read', 'points.adjust', 'commands.read']));
    const link = wrapper.get('#collaboration-link');
    expect((link.element as HTMLInputElement).value).toContain(`/remote/v1/join#inviteId=invite-id&secret=${privateSecret}`);
    expect(link.attributes('type')).toBe('password'); expect(wrapper.text()).not.toContain(privateSecret);
    expect(wrapper.findAll('a').some((a) => a.attributes('href')?.includes(privateSecret))).toBe(false);
    expect(toCanvas).not.toHaveBeenCalled();
    await button('Show QR code').trigger('click'); await vi.waitFor(() => expect(toCanvas).toHaveBeenCalledOnce());
    const target = toCanvas.mock.calls[0]?.[0] as HTMLCanvasElement | undefined;
    await button('Done').trigger('click'); await flushPromises();
    expect(wrapper.find('#collaboration-link').exists()).toBe(false); expect(wrapper.find('canvas').exists()).toBe(false);
    expect(target?.width).toBe(0); expect(target?.height).toBe(0);
  });

  it('shows a request as a highlighted row and requires reviewing the exact grant before approval', async () => {
    const status = collaborationStatus(); let approved = false;
    const request = { requestId: 'compare-this-request-id', clientLabel: '<img src=x onerror=alert(1)>', clientKind: 'desktop', grant: viewerGrant(), state: 'pending', expiresAt: Date.now() + 60000 };
    const { wrapper, calls, button, dialog } = mountPanel((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.requests.list') return { requests: approved ? [] : [request], nextAfter: null };
      if (method === 'collaboration.requests.approve') { approved = true; return { ok: true }; }
      return emptyCollaborationPage(method);
    });
    await flushPromises(); expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.get('.access-matrix__row--pending').text()).toContain('compare-this-request-id');
    await button('Approve device').trigger('click'); await flushPromises();
    expect(calls.some((c) => c.method === 'collaboration.requests.approve')).toBe(false);
    expect(dialog().text()).toContain(request.requestId);
    expect(dialog().text()).toContain('live.read');
    await dialog().findAll('button').find((b) => b.text() === 'Apply')!.trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'collaboration.requests.approve')).toEqual([{ method: 'collaboration.requests.approve', params: { id: request.requestId } }]);
    expect(wrapper.findAll('button').some((b) => b.text() === 'Approve device')).toBe(false);
  });

  it('edits a person in the matrix and only writes the new grant after the exact grant is confirmed', async () => {
    const status = collaborationStatus();
    const { wrapper, calls, button, cell, dialog } = mountPanel((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.principals.list') return { principals: [person], nextAfter: null };
      if (method === 'collaboration.principals.get') return copy(person);
      if (method === 'collaboration.grants.set') return { ok: true };
      return emptyCollaborationPage(method);
    });
    await flushPromises();
    await cell('Moderator PC, Points: None').trigger('click'); await flushPromises();
    expect(button('Apply permissions').attributes('disabled')).toBeDefined();
    await wrapper.findAll('[role="radio"]').find((node) => node.text() === 'Manage')!.trigger('click');
    expect(cell('Moderator PC, Points: Manage').exists()).toBe(true);
    expect(wrapper.get('.access-matrix__row--unsaved').text()).toContain('Moderator PC');
    expect(calls.some((c) => c.method === 'collaboration.grants.set')).toBe(false);
    await button('Apply permissions').trigger('click'); await flushPromises();
    expect(dialog().text()).toContain('points.adjust');
    expect(calls.some((c) => c.method === 'collaboration.grants.set')).toBe(false);
    await dialog().findAll('button').find((b) => b.text() === 'Apply')!.trigger('click'); await flushPromises();
    const write = calls.find((c) => c.method === 'collaboration.grants.set');
    expect(write?.params?.['expectedRevision']).toBe('1');
    expect((write?.params?.['grant'] as { capabilities: string[] }).capabilities).toEqual(expect.arrayContaining(['live.read', 'points.read', 'points.adjust', 'commands.read']));
  });

  it('keeps a configuration draft when the server rejects an outdated revision', async () => {
    const status = collaborationStatus();
    const { wrapper, calls, button } = mountPanel((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.configure') { status.configurationRevision = '2'; status.configuration.bindAddress = '127.0.0.1:9999'; throw new ControlCallError('revision_conflict', 'Configuration changed'); }
      return emptyCollaborationPage(method);
    });
    await flushPromises(); await button('Connection settings').trigger('click');
    await wrapper.get('#collaboration-bind').setValue('127.0.0.1:8999');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect((wrapper.get('#collaboration-bind').element as HTMLInputElement).value).toBe('127.0.0.1:8999');
    expect(wrapper.text()).toContain('Your edits were kept');
    expect(calls.filter((c) => c.method === 'collaboration.configure')).toHaveLength(1);
    expect(calls.find((c) => c.method === 'collaboration.configure')?.params?.['expectedRevision']).toBe('1');
    expect(button('Apply configuration').attributes('disabled')).toBeDefined();
    await button('Use latest configuration').trigger('click');
    expect((wrapper.get('#collaboration-bind').element as HTMLInputElement).value).toBe('127.0.0.1:9999');
  });
});
