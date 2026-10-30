import { describe, expect, test } from 'vitest';
import { mount } from '@vue/test-utils';
import RankingWidget from './RankingWidget.vue';
import type { RankingSnapshot } from '../shared/ranking-controller.ts';
const snapshot: RankingSnapshot = { rankingId: 'r', schemaVersion: 1, creatorId: 'c', roomId: 'room', revision: 1, integrityStatus: 'degraded', paused: true, rejected: 1, dropped: 0, limited: 0, cooldownRejected: 0, entries: [{ key: 'user-1', label: 'Alice', avatarUrl: 'https://cdn.example/alice.png', rank: 1, score: 10 }, { key: 'user-2', label: 'Bob', rank: 2, score: 5 }], totalScore: 15, totalParticipants: 2 };
describe('ranking widget', () => {
  test('accounts for scores outside the displayed entries and offers chart controls', async () => {
    const partial = { ...snapshot, totalScore: 20, totalParticipants: 3 };
    const wrapper = mount(RankingWidget, { props: { snapshot: partial, visualization: 'donut', status: 'connected', debug: false, showPercentages: true } });
    expect(wrapper.find('.chart').text()).toBe('20');
    expect(wrapper.find('.other').text()).toContain('5');
    expect(wrapper.text()).toContain('50.0%');
    expect(wrapper.text()).toContain('25.0%');
    await wrapper.setProps({ showLegend: false });
    expect(wrapper.find('.rows').exists()).toBe(false);
    expect(wrapper.find('.chart').text()).toBe('20');
    await wrapper.setProps({ showLegend: true, showLabels: false });
    expect(wrapper.text()).not.toContain('Alice');
    expect(wrapper.text()).toContain('10');
  });
  test('shows poll shares against the complete snapshot total', () => {
    const wrapper = mount(RankingWidget, { props: { snapshot: { ...snapshot, totalScore: 20, totalParticipants: 3 }, visualization: 'poll', status: 'connected', debug: false } });
    expect(wrapper.text()).toContain('Poll');
    expect(wrapper.text()).toContain('50.0%');
    expect(wrapper.find('.poll-bar').attributes('style')).toContain('width: 50%');
  });
  test('renders one backend snapshot in each supported mode', () => {
    for (const visualization of ['ranking', 'bar', 'pie', 'donut'] as const) {
      const wrapper = mount(RankingWidget, { props: { snapshot, visualization, status: 'connected', debug: false } });
      expect(wrapper.text()).toContain('Alice');
      expect(wrapper.text()).toContain('10');
      expect(wrapper.find('img').attributes('src')).toBe('https://cdn.example/alice.png');
      expect(wrapper.text()).toContain('Paused');
      expect(wrapper.text()).toContain('Incomplete');
      expect(wrapper.find('.chart').exists()).toBe(visualization === 'pie' || visualization === 'donut');
      expect(wrapper.find('.bar').exists()).toBe(visualization === 'bar');
      wrapper.unmount();
    }
  });
});
