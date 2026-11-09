import { expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import SyncConflictReview from './SyncConflictReview.vue';
import SyncStatusSummary from './SyncStatusSummary.vue';
test('conflict actions preserve key and local/cloud choice and are disabled while working', async () => {
  const wrapper = mountWithPlugins(SyncConflictReview, {props:{conflicts:['settings.theme'],working:false}});
  const buttons=wrapper.findAll('button');
  await buttons[0]!.trigger('click'); await buttons[1]!.trigger('click');
  expect(wrapper.emitted('resolve')).toEqual([['settings.theme',true],['settings.theme',false]]);
  await wrapper.setProps({working:true});
  expect(wrapper.findAll('button').every(b=>b.attributes('disabled') !== undefined)).toBe(true);
  expect(wrapper.text()).not.toContain('Base version');
});
test('summary represents unknown last sync and exact pending count without invented backlog', () => {
  const wrapper=mountWithPlugins(SyncStatusSummary, {props:{status:{enabled:true,signedIn:true,syncing:false,pending:3,conflicts:[],lastSyncedAt:null}}});
  expect(wrapper.text()).toContain('3');
  expect(wrapper.text()).toContain('Not synced yet');
  expect(wrapper.text()).not.toContain('Pending application');
});
