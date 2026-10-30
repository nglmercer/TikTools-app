import { describe, expect, test } from 'vitest';
import { mount } from '@vue/test-utils';
import RevealWidget from './RevealWidget.vue';
import { RevealController, parseRevealResult } from '../shared/reveal-controller.ts';
const event = { topic: 'competition.result', data: { result: { schemaVersion: 1, competitionId: 'poll', rankingId: 'r', generation: 1, selection: { mode: 'weighted_random', winner: 'b', eligible: [{ key: 'a', weight: 1 }, { key: 'b', weight: 3 }] } } } };
describe('committed result reveal', () => {
  test('reuses the committed selection and ignores refresh duplicates', async () => {
    const values: unknown[] = [];
    const controller = new RevealController('r', (value) => values.push(value));
    expect(controller.handleEnvelope(event)).toBe(true);
    expect(controller.handleEnvelope(event)).toBe(false);
    const result = parseRevealResult(event)!;
    const wrapper = mount(RevealWidget, { props: { result, status: 'connected', debug: false } });
    expect(wrapper.text()).toContain('Revealing winner');
    wrapper.find('.wheel').trigger('animationend');
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain('Winner: b');
    expect(wrapper.find('.wheel').attributes('style')).toContain('--rotation: 1575deg');
    controller.clear();
    expect(controller.handleEnvelope(event)).toBe(true);
    expect(values).toHaveLength(3);
  });
  test('filters results by competition identity', () => {
    const values: unknown[] = [];
    const controller = new RevealController('r', (value) => values.push(value), 'poll');
    const other = { ...event, data: { result: { ...event.data.result, competitionId: 'other' } } };
    expect(controller.handleEnvelope(other)).toBe(false);
    expect(controller.handleEnvelope(event)).toBe(true);
    controller.clear();
    expect(controller.handleEnvelope(other)).toBe(false);
    expect(controller.handleEnvelope(event)).toBe(true);
  });
  test('rejects invalid winners and unsafe weights', () => {
    expect(parseRevealResult({ ...event, data: { result: { ...event.data.result, selection: { ...event.data.result.selection, winner: 'missing' } } } })).toBeNull();
    expect(parseRevealResult({ ...event, data: { result: { ...event.data.result, selection: { ...event.data.result.selection, eligible: [{ key: 'a', weight: Number.MAX_SAFE_INTEGER + 1 }] } } } })).toBeNull();
  });
  test('includes omitted candidates in the wheel without changing the stored winner', () => {
    const result = parseRevealResult({ ...event, data: { result: { ...event.data.result, eligibleCount: 4, otherWeight: 6 } } })!;
    const wrapper = mount(RevealWidget, { props: { result, status: 'connected', debug: false } });
    expect(wrapper.find('.wheel').attributes('style')).toContain('--rotation: 1710deg');
    expect(wrapper.text()).toContain('Other: 2 participants');
    expect(parseRevealResult({ ...event, data: { result: { ...event.data.result, eligibleCount: 1, otherWeight: 6 } } })).toBeNull();
  });
});
