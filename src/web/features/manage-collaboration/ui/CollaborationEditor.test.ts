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
  return { ...fake, wrapper, button: (text: string) => wrapper.findAll('button').find((node) => node.text() === text)! };
}
describe('owner collaboration panel', () => {
  it('keeps network drafts while browsing other areas and does not write on navigation', async () => {
    const status = collaborationStatus();
    const { wrapper, calls, button } = mountPanel(method => method === 'collaboration.status' ? copy(status) : emptyCollaborationPage(method));
    await flushPromises();
    expect(wrapper.get('#collaboration-bind').isVisible()).toBe(false);
    await button('Network and security').trigger('click');
    await wrapper.get('#collaboration-bind').setValue('127.0.0.1:8999');
    await button('Status and access').trigger('click');
    expect(wrapper.get('#collaboration-bind').isVisible()).toBe(false);
    await button('Network and security').trigger('click');
    expect(wrapper.get('#collaboration-bind').isVisible()).toBe(true);
    expect((wrapper.get('#collaboration-bind').element as HTMLInputElement).value).toBe('127.0.0.1:8999');
    expect(calls.some(call => call.method === 'collaboration.configure')).toBe(false);
  });

  it('starts with reads only, shares a fragment link and generates QR only on request', async () => {
    const status = collaborationStatus(); const privateSecret = 'b'.repeat(64);
    const { wrapper, calls, button } = mountPanel((method) => method === 'collaboration.status' ? copy(status) : method === 'collaboration.invites.create' ? { inviteId: 'invite-id', secret: privateSecret, expiresAt: Date.now() + 60000 } : emptyCollaborationPage(method));
    await flushPromises();
    expect(calls.some((c) => c.method === 'collaboration.configure')).toBe(false);
    expect(calls.some((c) => c.method === 'collaboration.invites.create')).toBe(false);
    await button('Create private invitation').trigger('click'); await flushPromises();
    const link = wrapper.get('#collaboration-link');
    expect((link.element as HTMLInputElement).value).toContain(`/remote/v1/join#inviteId=invite-id&secret=${privateSecret}`);
    expect(link.attributes('type')).toBe('password'); expect(wrapper.text()).not.toContain(privateSecret);
    expect(wrapper.findAll('a').some((a) => a.attributes('href')?.includes(privateSecret))).toBe(false);
    expect(toCanvas).not.toHaveBeenCalled();
    await button('Show QR code').trigger('click'); await vi.waitFor(() => expect(toCanvas).toHaveBeenCalledOnce());
    const target = toCanvas.mock.calls[0]?.[0] as HTMLCanvasElement | undefined;
    await button('Hide private link').trigger('click'); await flushPromises();
    expect(wrapper.find('#collaboration-link').exists()).toBe(false); expect(wrapper.find('canvas').exists()).toBe(false);
    expect(target?.width).toBe(0); expect(target?.height).toBe(0);
  });
  it('requires reviewing the exact request before approval and escapes an untrusted label', async () => {
    const status = collaborationStatus(); let approved = false;
    const request = { requestId: 'compare-this-request-id', clientLabel: '<img src=x onerror=alert(1)>', clientKind: 'desktop', grant: viewerGrant(), state: 'pending', expiresAt: Date.now() + 60000 };
    const { wrapper, calls, button } = mountPanel((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.requests.list') return { requests: approved ? [] : [request], nextAfter: null };
      if (method === 'collaboration.requests.approve') { approved = true; return { ok: true }; }
      return emptyCollaborationPage(method);
    });
    await flushPromises(); expect(wrapper.find('img').exists()).toBe(false);
    await button('Approve device').trigger('click'); await flushPromises();
    expect(calls.some((c) => c.method === 'collaboration.requests.approve')).toBe(false);
    expect(wrapper.get('dialog').text()).toContain(request.requestId);
    expect(wrapper.get('dialog').text()).toContain('live.read');
    await wrapper.get('dialog').findAll('button').find((b) => b.text() === 'Apply')!.trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'collaboration.requests.approve')).toEqual([{ method: 'collaboration.requests.approve', params: { id: request.requestId } }]);
    expect(wrapper.findAll('button').some((b) => b.text() === 'Approve device')).toBe(false);
  });
  it('keeps a configuration draft when the server rejects an outdated revision', async () => {
    const status = collaborationStatus();
    const { wrapper, calls, button } = mountPanel((method) => {
      if (method === 'collaboration.status') return copy(status);
      if (method === 'collaboration.configure') { status.configurationRevision = '2'; status.configuration.bindAddress = '127.0.0.1:9999'; throw new ControlCallError('revision_conflict', 'Configuration changed'); }
      return emptyCollaborationPage(method);
    });
    await flushPromises(); await wrapper.get('#collaboration-bind').setValue('127.0.0.1:8999');
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
