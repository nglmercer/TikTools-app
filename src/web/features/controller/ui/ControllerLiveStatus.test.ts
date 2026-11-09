import { mount } from '@vue/test-utils';
import { ref } from 'vue';
import { expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';
import { createLiveStatusModel } from '../model/live.ts';
import ControllerLiveStatus from './ControllerLiveStatus.vue';

it('shows the authorized host connection without exposing remote controls', async () => {
  i18n.global.locale.value = 'es';
  const status = ref<ControllerStatus>({ epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host', hostInstanceId: 'instance', principalId: 'moderator', sessionId: 'device', capabilities: ['live.read'], features: {} } });
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(status.value)) : { epoch: '1', connectionId: 'connection', connected: true, uniqueId: 'creator', native: true });
  const model = createLiveStatusModel(client, { status, generation: () => 1, stopped: () => false });
  const wrapper = mount(ControllerLiveStatus, { props: { model }, global: { plugins: [i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('Conectado al LIVE');
    expect(wrapper.text()).toContain('Creador: creator');
    expect(wrapper.findAll('button')).toHaveLength(0);
    status.value.phase = 'reconnecting'; await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).not.toContain('creator');
  } finally { wrapper.unmount(); }
});
