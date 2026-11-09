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
    events:['chat', 'gift', 'like', 'member', 'social'].map((kind, i) => ({ cursor:String(i + 1), kind, author:'alice', authorLookupId:'alice', nickname:null, text:i === 0 ? '<script>unsafe</script>' : 'activity' })) };
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

it('offers a points lookup only for permitted feed entries', async () => {
  i18n.global.locale.value = 'en';
  const status = ref<ControllerStatus>({ epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device', capabilities:['live.read'], features:{} } });
  const events = ['alice', 'bob'].map((author, i) => ({ cursor:String(i + 1), kind:'chat', author, authorLookupId:author, nickname:null, text:'hello' }));
  const reply = { epoch:'1', connectionId:'connection', oldestCursor:'1', latestCursor:'2', nextCursor:'2', gap:false, truncated:false, events };
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(status.value)) : reply);
  const model = createLiveFeedModel(client, { status, generation:() => 1, stopped:() => false });
  const wrapper = mount(ControllerLiveFeed, { props:{ model, canLookupPoints: (event) => event.author === 'alice' }, global:{ plugins:[i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.findAll('button').map((button) => button.text())).toEqual(['Look up points for @alice']);
    await wrapper.get('button').trigger('click');
    expect(wrapper.emitted('lookupPoints')).toEqual([[events[0]]]);
    expect(model.events.value).toEqual(events);
  } finally { wrapper.unmount(); }
});

it('lets a restricted moderator switch between allowed creator histories', async () => {
  i18n.global.locale.value = 'en';
  const status = ref<ControllerStatus>({ epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device', capabilities:['live.read'], features:{ creatorFeed:true }, resourceRestrictions:{ live:['bob', 'alice'] } } });
  const { client } = createFakeControlClient((method, params) => {
    if (method === 'controller.status') return JSON.parse(JSON.stringify(status.value));
    const creator = String(params?.['creatorId']);
    return { epoch:'1', connectionId:'connection', creatorId:creator, streamId:'a'.repeat(32), oldestCursor:'1', latestCursor:'1', nextCursor:'1', gap:false, truncated:false,
      events:[{ cursor:'1', kind:'chat', author:'viewer', authorLookupId:null, nickname:null, text:`${creator} activity` }] };
  });
  const model = createLiveFeedModel(client, { status, generation:() => 1, stopped:() => false });
  const wrapper = mount(ControllerLiveFeed, { props:{ model }, global:{ plugins:[i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('alice activity');
    const selector = wrapper.findAll('select')[0]!;
    expect(selector.findAll('option').map((item) => item.text())).toEqual(['alice', 'bob']);
    await selector.setValue('bob');
    expect(wrapper.text()).not.toContain('alice activity');
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('bob activity');
    expect(wrapper.text()).not.toContain('alice activity');
  } finally { wrapper.unmount(); }
});

it('keeps creator selection available when one creator feed fails', async () => {
  i18n.global.locale.value = 'en';
  const status = ref<ControllerStatus>({ epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device', capabilities:['live.read'], features:{ creatorFeed:true }, resourceRestrictions:{ live:['alice', 'bob'] } } });
  const { client } = createFakeControlClient((method, params) => {
    if (method === 'controller.status') return JSON.parse(JSON.stringify(status.value));
    if (params?.['creatorId'] === 'alice') throw new Error('feed unavailable');
    return { epoch:'1', connectionId:'connection', creatorId:'bob', streamId:'b'.repeat(32), oldestCursor:'1', latestCursor:'1', nextCursor:'1', gap:false, truncated:false,
      events:[{ cursor:'1', kind:'chat', author:'viewer', authorLookupId:null, nickname:null, text:'bob activity' }] };
  });
  const model = createLiveFeedModel(client, { status, generation:() => 1, stopped:() => false });
  const wrapper = mount(ControllerLiveFeed, { props:{ model }, global:{ plugins:[i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.get('[role="alert"]').text()).toContain('unavailable');
    await wrapper.get('select').setValue('bob');
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('bob activity');
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  } finally { wrapper.unmount(); }
});
