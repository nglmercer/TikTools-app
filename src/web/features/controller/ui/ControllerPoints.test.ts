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
    const dialog = () => wrapper.get('dialog[open]');
    expect(dialog().text()).toContain('https://host.example'); expect(dialog().text()).toContain('host-A');
    expect(dialog().text()).toContain('new-viewer · -3 Gems'); expect(dialog().text()).toContain('The adjustment will create one.');
    expect(dialog().text()).toContain('balance at acceptance');
    await dialog().findAll('button').find((node) => node.text() === 'Cancel')!.trigger('click'); await flushPromises();
    expect(calls.some((c) => c.method === 'controller.points.command')).toBe(false);
    await wrapper.get('.controller__pointTarget form').trigger('submit'); await flushPromises();
    await dialog().findAll('button').find((node) => node.text() === 'Adjust points')!.trigger('click'); await flushPromises();
    expect(calls.filter((c) => c.method === 'controller.points.command')).toHaveLength(1);
    expect(calls.find((c) => c.method === 'controller.points.command')?.params).toEqual({ epoch: '1', connectionId: 'connection', uniqueId: 'new-viewer', delta: -3, currencyName: 'Gems' });
    expect(wrapper.text()).toContain('new changes are blocked');
    expect(wrapper.get('input[type=number]').attributes('disabled')).toBeDefined();
    expect(calls.every((c) => c.method.startsWith('controller.') && c.method !== 'controller.forward')).toBe(true);
  } finally { wrapper.unmount(); }
});
