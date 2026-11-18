import { testMessage } from '@/shared/i18n/test-message.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import type { BehaviorRun } from '../../../../automation/behavior/types.ts';
import RunEventDataView from './RunEventDataView.vue';

enableAutoUnmount(afterEach);

function run(id: string, comment?: string): BehaviorRun {
  return {
    id,
    at: 1,
    status: 'ok',
    actionName: 'Sonora: Search and play',
    summary: 'ok',
    durationMs: 1,
    test: true,
    logs: [],
    ...(comment === undefined ? {} : { eventData: { comment } }),
  };
}

describe('RunEventDataView', () => {
  it('shows the selected payload inline and navigates between runs', async () => {
    const runs = [run('a', '!play one'), run('b', '!play two')];
    const wrapper = mountWithPlugins(RunEventDataView, { props: { runs, index: 0 } });
    expect(wrapper.get('pre').text()).toContain('!play one');
    expect(wrapper.get('.run-event-data__position').text()).toBe('1 / 2');
    await wrapper
      .get(`button[aria-label="${testMessage('en', 'automations.eventData.next')}"]`)
      .trigger('click');
    expect(wrapper.emitted('navigate')?.[0]).toEqual([1]);
  });

  it('shows the empty message when the run carries no event data', () => {
    const wrapper = mountWithPlugins(RunEventDataView, { props: { runs: [run('a')], index: 0 } });
    expect(wrapper.text()).toContain(testMessage('en', 'automations.eventData.empty'));
    expect(wrapper.find('pre').exists()).toBe(false);
  });
});
