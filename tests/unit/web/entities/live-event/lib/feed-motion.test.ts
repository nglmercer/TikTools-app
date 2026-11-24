import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { defineComponent, nextTick, ref, type Ref } from 'vue';
import { mount } from '@vue/test-utils';
import type { DisplayEvent } from '@/types.ts';

const motionState = vi.hoisted(() => ({ reduced: undefined as unknown as Ref<boolean> }));
vi.mock('@vueuse/motion', async () => {
  const { ref: createRef } = await import('vue');
  motionState.reduced = createRef(false);
  return { useReducedMotion: () => motionState.reduced };
});

import { useFeedMotion } from '../../../../../../src/web/entities/live-event/lib/feed-motion.ts';

function event(id: number, receivedAt: number): DisplayEvent {
  return { id, kind: 'chat', author: `user${id}`, text: 'hola', receivedAt } as DisplayEvent;
}

function host(events: Ref<readonly DisplayEvent[]>) {
  let motion!: ReturnType<typeof useFeedMotion>;
  const wrapper = mount(defineComponent({
    setup() {
      motion = useFeedMotion(events);
      return () => null;
    },
  }));
  return { wrapper, motion };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-01-01T00:00:00Z'));
  motionState.reduced.value = false;
});

afterEach(() => {
  vi.useRealTimers();
});

it('uses a short opacity fade for normal feed updates', () => {
  const { motion, wrapper } = host(ref<DisplayEvent[]>([]));
  expect(motion.row.value.enter).toMatchObject({ opacity: 1, transition: { duration: 120 } });
  expect(motion.gift.value.enter).toEqual(motion.row.value.enter);
  wrapper.unmount();
});

it('renders static rows when the user prefers reduced motion', async () => {
  const { motion, wrapper } = host(ref<DisplayEvent[]>([]));
  motionState.reduced.value = true;
  await nextTick();
  expect(motion.row.value.initial).toEqual({ opacity: 1 });
  expect(motion.row.value.enter).toEqual({ opacity: 1 });
  expect(motion.gift.value.enter).toEqual({ opacity: 1 });
  wrapper.unmount();
});

it('suppresses motion during a burst and restores it after the window', async () => {
  const now = Date.now();
  const events = ref<DisplayEvent[]>([event(1, now), event(2, now)]);
  const { motion, wrapper } = host(events);
  expect(motion.row.value.enter).toMatchObject({ transition: { duration: 120 } });

  events.value = [event(1, now), event(2, now), event(3, now)];
  await nextTick();
  expect(motion.row.value.enter).toEqual({ opacity: 1 });

  vi.advanceTimersByTime(200);
  await nextTick();
  expect(motion.row.value.enter).toMatchObject({ transition: { duration: 120 } });
  wrapper.unmount();
});

it('clears the pending burst timer on unmount', async () => {
  const now = Date.now();
  const events = ref<DisplayEvent[]>([event(1, now), event(2, now)]);
  const { wrapper } = host(events);
  events.value = [event(1, now), event(2, now), event(3, now)];
  await nextTick();
  expect(vi.getTimerCount()).toBe(1);
  wrapper.unmount();
  expect(vi.getTimerCount()).toBe(0);
});
