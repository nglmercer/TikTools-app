import { mount } from '@vue/test-utils';
import { ref } from 'vue';
import { expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ControllerStatus } from '../model/controller.ts';
import { createCompetitionModel } from '../model/competitions.ts';
import ControllerCompetitions from './ControllerCompetitions.vue';

it('renders authorized status as text and hides it on capability removal', async () => {
  const status = ref<ControllerStatus>({ epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device',
      capabilities:['competitions.read'], features:{ competitionSummary:true }, resourceRestrictions:{ competition:['allowed'] } } });
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(status.value)) :
    { epoch:'1', connectionId:'connection', nextAfter:null, competitions:[{ id:'allowed', name:'<script>private</script>', kind:'poll', lifecycle:'active', generation:'1', deadlineMs:null, remainingMs:null }] });
  const model = createCompetitionModel(client, { status, generation:() => 1, stopped:() => false });
  const wrapper = mount(ControllerCompetitions, { props:{ model }, global:{ plugins:[i18n] } });
  try {
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('<script>private</script>');
    expect(wrapper.find('script').exists()).toBe(false);
    expect(wrapper.text()).toContain('Active');
    expect(wrapper.findAll('button')).toHaveLength(2);
    status.value.hello!.capabilities = [];
    await model.refresh(status.value, 1); await wrapper.vm.$nextTick();
    expect(wrapper.text()).not.toContain('private');
  } finally { wrapper.unmount(); }
});
