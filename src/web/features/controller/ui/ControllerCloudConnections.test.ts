import { mount, flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import { expect, it, vi } from 'vitest';
import { activateLocale, i18n } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';
import ControllerCloudConnections from './ControllerCloudConnections.vue';

it('shows approved teams and requests Cloud access without an HTTPS address or host runtime calls', async () => {
  await activateLocale('es');
  const status = ref<ControllerStatus>({ epoch: '1', viewRevision: '1', phase: 'idle', profile: null, origin: null, errorCode: null, hello: null, cloudSupported: true });
  const model = { status, inventory: vi.fn(async () => {}), refresh: vi.fn(async () => {}), connect: vi.fn(async () => {}), disconnect: vi.fn(async () => {}) };
  const secret = 'private-cloud-invitation-marker-123456';
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.cloud.status') return { account: { state: 'signed-in', email: 'moderator@example.test', accountId: 'moderator', origin: 'https://cloud.example' }, deviceId: 'device' };
    if (method === 'controller.cloud.teams') return { items: [{ id: 'host', name: 'Mi LIVE', role: 'moderator', clientKind: 'desktop' }] };
    if (method === 'controller.cloud.join') { status.value = { ...status.value, epoch: '2', phase: 'cloud-pairing', cloudPairing: { profile: 'desk', hostId: 'host', requestId: 'request' } }; return status.value.cloudPairing; }
    if (method === 'controller.cloud.pair.poll') return { approved: false };
    throw new Error(`Unexpected native call ${method}`);
  });
  const wrapper = mount(ControllerCloudConnections, { props: { model }, global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  try {
    await flushPromises();
    expect(wrapper.text()).toContain('Mi LIVE');
    expect(wrapper.findAll('input[type="radio"]')).toHaveLength(1);
    expect(wrapper.find('input[type="url"]').exists()).toBe(false);
    await wrapper.findAll('[role="tab"]').find((tab) => tab.text() === 'Unirme a un equipo')!.trigger('click');
    await wrapper.get('input[pattern]').setValue('desk');
    await wrapper.get('input[type="password"]').setValue(secret);
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(wrapper.text()).toContain('Esperando aprobación');
    expect(wrapper.html()).not.toContain(secret);
    expect(calls.find((call) => call.method === 'controller.cloud.join')?.params?.['token']).toBe(secret);
    expect(calls.every((call) => call.method.startsWith('controller.cloud.'))).toBe(true);
  } finally { wrapper.unmount(); }
});
