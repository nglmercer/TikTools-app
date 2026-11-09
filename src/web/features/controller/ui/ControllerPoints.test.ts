import { mount, flushPromises } from '@vue/test-utils';
import { expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { ControllerStatus, PointsPage } from '../model/controller.ts';
import ControllerRoot from './ControllerRoot.vue';

it('reviews host, unit, signed amount and viewer creation, then blocks all writes after a lost acknowledgement', async () => {
  i18n.global.locale.value = 'en';
  const current: ControllerStatus = { epoch: '1', viewRevision: '1', pointsRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', grantRevision: '1', hostId: 'host-A', principalId: 'mod', sessionId: 'device', hostInstanceId: 'instance', capabilities: ['points.read', 'points.adjust', 'commands.read'], features: { pointsProjection: true, pointsAdjust: true } },
    commandJournal: { binding: { profileId: 'desk', origin: 'https://host.example', hostId: 'host-A', principalId: 'mod', sessionId: 'device' }, pending: null, recent: [], unavailable: false } };
  const points: PointsPage = { epoch: '1', pointsRevision: '1', loading: false, errorCode: null, currencyName: 'Gems', rows: [], selectedId: null, selected: null, selectedVersion: null };
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(current);
    if (method === 'controller.workflows') return { epoch: current.epoch, viewRevision: current.viewRevision, rows: [], next: null };
    if (method === 'controller.points') return structuredClone(points);
    if (method === 'controller.points.select') {
      points.selectedId = String(params?.['uniqueId']); points.selectedVersion = { kind: 'viewer', scope: 'host', id: points.selectedId, revision: '0' };
      points.pointsRevision = current.pointsRevision = '2'; return {};
    }
    if (method === 'controller.points.command') {
      current.commandJournal!.pending = { commandId: 'native-generated', operation: 'adjust-points', resourceId: 'new-viewer', delta: -3, currencyName: 'Gems', hostInstanceId: 'instance', createdAt: 1 };
      current.viewRevision = '2';
      throw new ControlCallError('transport', 'Acknowledgement lost');
    }
    throw new Error(`Unexpected native call: ${method}`);
  });
  const wrapper = mount(ControllerRoot, { global: { plugins: [i18n], provide: { [controlClientKey as symbol]: client } } });
  try {
    await flushPromises();
    await wrapper.get('.controller__pointSearch input').setValue('@new-viewer');
    await wrapper.get('.controller__pointSearch').trigger('submit'); await flushPromises();
    expect(wrapper.text()).toContain('The adjustment will create one.');
    await wrapper.get('input[type=number]').setValue('-3');
    await wrapper.get('.controller__pointTarget form').trigger('submit'); await flushPromises();
    expect(calls.filter((c) => c.method === 'controller.points.command')).toHaveLength(1);
    expect(calls.find((c) => c.method === 'controller.points.command')?.params).toEqual({ epoch: '1', connectionId: 'connection', uniqueId: 'new-viewer', delta: -3, currencyName: 'Gems' });
    expect(wrapper.text()).toContain('new changes are blocked');
    expect(wrapper.get('input[type=number]').attributes('disabled')).toBeDefined();
    expect(calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward')).toBe(true);
  } finally { wrapper.unmount(); }
});

it('opens a permitted LIVE author in points without carrying an earlier amount or sending a write', async () => {
  i18n.global.locale.value = 'en';
  const current: ControllerStatus = { epoch:'1', viewRevision:'1', pointsRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host-A', hostInstanceId:'instance', principalId:'mod', sessionId:'device',
      capabilities:['live.read','points.read','points.adjust','commands.read'], features:{ pointsProjection:true, pointsAdjust:true }, resourceRestrictions:{ viewer:['alice','bob','viewer'] } },
    commandJournal:{ binding:{ profileId:'desk', origin:'https://host.example', hostId:'host-A', principalId:'mod', sessionId:'device' }, pending:null, recent:[], unavailable:false } };
  const points: PointsPage = { epoch:'1', pointsRevision:'1', loading:false, errorCode:null, currencyName:'Gems', rows:[], selectedId:null, selected:null, selectedVersion:null };
  const events = ['bob','mallory','viewer'].map((author, i) => ({ cursor:String(i + 1), kind:'chat', author, authorLookupId:author === 'viewer' ? null : author, nickname:null, text:'hello' }));
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(current);
    if (method === 'controller.workflows') return { epoch:'1', viewRevision:'1', rows:[], next:null };
    if (method === 'controller.live.status') return { epoch:'1', connectionId:'connection', connected:true, uniqueId:'creator', native:true };
    if (method === 'controller.live.feed') return { epoch:'1', connectionId:'connection', oldestCursor:'1', latestCursor:'3', nextCursor:'3', gap:false, truncated:false, events };
    if (method === 'controller.points') return structuredClone(points);
    if (method === 'controller.points.select') {
      points.selectedId = String(params?.['uniqueId']); points.selectedVersion = { kind:'viewer', scope:'host', id:points.selectedId, revision:'0' };
      points.pointsRevision = current.pointsRevision = String(Number(current.pointsRevision) + 1); return {};
    }
    throw new Error(`Unexpected native call: ${method}`);
  });
  let controllerChanged = () => {};
  client.onTopic = (topic, handler) => { if (topic === 'controller.changed') controllerChanged = () => handler(undefined as never); return () => {}; };
  const wrapper = mount(ControllerRoot, { global:{ plugins:[i18n], provide:{ [controlClientKey as symbol]:client } } });
  try {
    await flushPromises();
    await wrapper.get('.controller__pointSearch input').setValue('alice');
    await wrapper.get('.controller__pointSearch').trigger('submit'); await flushPromises();
    await wrapper.get('.controller__pointTarget input[type=number]').setValue('7');
    const feed = wrapper.get('.live-feed');
    const buttons = feed.findAll('button');
    expect(buttons.map((button) => button.text())).toEqual(['Look up points for @bob']);
    expect(feed.text()).toContain('The author ID cannot be confirmed; look it up manually.');
    await buttons[0]!.trigger('click'); await flushPromises();
    expect((wrapper.get('.controller__pointSearch input').element as HTMLInputElement).value).toBe('bob');
    expect(wrapper.get('.controller__pointTarget').text()).toContain('Selected viewer: bob');
    expect((wrapper.get('.controller__pointTarget input[type=number]').element as HTMLInputElement).value).toBe('');
    expect(calls.filter((call) => call.method === 'controller.points.select').map((call) => call.params)).toEqual([
      { epoch:'1', uniqueId:'alice' }, { epoch:'1', uniqueId:'bob' },
    ]);
    expect(calls.some((call) => call.method === 'controller.points.command')).toBe(false);
    await wrapper.get('.controller__pointTarget input[type=number]').setValue('9');
    current.hello!.grantRevision = '2'; current.hello!.resourceRestrictions = { viewer:['alice'] };
    controllerChanged(); await flushPromises();
    expect((wrapper.get('.controller__pointSearch input').element as HTMLInputElement).value).toBe('');
    expect((wrapper.get('.controller__pointTarget input[type=number]').element as HTMLInputElement).value).toBe('');
    expect(wrapper.get('.live-feed').findAll('button')).toHaveLength(0);
    expect(calls.some((call) => call.method === 'controller.points.command')).toBe(false);
  } finally { wrapper.unmount(); }
});
