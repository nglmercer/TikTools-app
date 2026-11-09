import { mount } from '@vue/test-utils';
import { ref } from 'vue';
import { expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';
import { createLiveFeedModel } from '../model/live-feed.ts';
import ControllerLiveFeed from './ControllerLiveFeed.vue';

it('renders untrusted activity as text and clears it on revocation', async () => {
  i18n.global.locale.value = 'es';
  const status = ref<ControllerStatus>({ epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device', capabilities:['live.read'], features:{} } });
  const reply = { epoch:'1', connectionId:'connection', oldestCursor:'1', latestCursor:'5', nextCursor:'5', gap:true, truncated:false,
    events:['chat', 'gift', 'like', 'member', 'social'].map((kind, i) => ({ cursor:String(i + 1), kind, author:'alice', nickname:null, text:i === 0 ? '<script>unsafe</script>' : 'activity' })) };
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(status.value)) : { ...reply, connectionId: status.value.hello!.connectionId });
  const model = createLiveFeedModel(client, { status, generation:() => 1, stopped:() => false });
  const wrapper = mount(ControllerLiveFeed, { props:{ model }, global:{ plugins:[i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('<script>unsafe</script>'); expect(wrapper.find('script').exists()).toBe(false);
    expect(wrapper.text()).toContain('Se perdió parte de la actividad'); expect(wrapper.findAll('button')).toHaveLength(0);
    expect(wrapper.findAll('.kind').map((element) => element.text())).toEqual(['Chat', 'Regalo', 'Me gusta', 'Ingreso', 'Social']);
    await wrapper.find('select').setValue('gift');
    expect(wrapper.findAll('.kind').map((element) => element.text())).toEqual(['Regalo']);
    await wrapper.find('input').setValue('does not match');
    expect(wrapper.text()).toContain('Ninguna actividad visible coincide');
    await wrapper.find('input').setValue('ALICE');
    expect(wrapper.findAll('.kind').map((element) => element.text())).toEqual(['Regalo']);
    await wrapper.find('select').setValue('all');
    await wrapper.find('input').setValue('');
    i18n.global.locale.value = 'en'; await wrapper.vm.$nextTick();
    expect(wrapper.findAll('.kind').map((element) => element.text())).toEqual(['Chat', 'Gift', 'Like', 'Join', 'Social']);
    await wrapper.find('select').setValue('chat');
    await wrapper.find('input').setValue('previous host');
    status.value.hello!.connectionId = 'replacement';
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect((wrapper.find('select').element as HTMLSelectElement).value).toBe('all');
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('');
    await wrapper.find('input').setValue('private search');
    status.value.hello!.capabilities = []; await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).not.toContain('unsafe');
    status.value.hello!.capabilities = ['live.read']; await wrapper.vm.$nextTick();
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('');
  } finally { wrapper.unmount(); }
});
