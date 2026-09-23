import { describe, expect, test } from 'bun:test';

import {
  SubscribeController,
  type SubscribeAlert,
} from './subscribe-controller.ts';
import { createFakeClock } from './test-clock.ts';
import { makeTestGiftEnvelope, makeTestSubscribeEnvelope } from './test-events.ts';

function setup(options: { visibleMs?: number; queueCapacity?: number } = {}) {
  const fake = createFakeClock();
  const shown: SubscribeAlert[] = [];
  const hidden: string[] = [];
  const controller = new SubscribeController(
    {
      onShow: (alert) => void shown.push(alert),
      onHide: (id) => void hidden.push(id),
    },
    { visibleMs: options.visibleMs ?? 5000, queueCapacity: options.queueCapacity, clock: fake.clock },
  );
  return { fake, shown, hidden, controller };
}

describe('subscribe controller', () => {
  test('shows one alert, holds, then hides', () => {
    const { fake, shown, hidden, controller } = setup();
    expect(controller.handleEnvelope(makeTestSubscribeEnvelope())).toBe('shown');
    expect(shown).toHaveLength(1);
    const first = shown[0];
    if (first === undefined) throw new Error('expected a shown alert');
    expect(first.displayName).toBe('Viewer Name');
    expect(controller.currentAlert?.id).toBe(first.id);
    fake.advance(4999);
    expect(hidden).toHaveLength(0);
    fake.advance(1);
    expect(hidden).toEqual([first.id]);
    expect(controller.currentAlert).toBeNull();
  });

  test('carries the sender avatar, null when TikTok sent none', () => {
    const { fake, shown, controller } = setup();
    controller.handleEnvelope(
      makeTestSubscribeEnvelope({ user: { avatarUrl: 'https://cdn.example/sub.png' } }),
    );
    controller.handleEnvelope(makeTestSubscribeEnvelope({ user: { nickname: 'No Avatar' } }));
    fake.advance(5000);
    expect(shown.map((alert) => alert.avatarUrl)).toEqual(['https://cdn.example/sub.png', null]);
  });

  test('plain joins never alert, only member action 3', () => {
    const { shown, controller } = setup();
    expect(controller.handleEnvelope(makeTestSubscribeEnvelope({ action: 1 }))).toBe('ignored');
    expect(controller.handleEnvelope(makeTestSubscribeEnvelope({ action: 0 }))).toBe('ignored');
    expect(shown).toHaveLength(0);
    expect(controller.handleEnvelope(makeTestSubscribeEnvelope({ action: 3 }))).toBe('shown');
    expect(shown).toHaveLength(1);
  });

  test('ignores history, duplicates, and non-subscribe events', () => {
    const { shown, controller } = setup();
    expect(controller.handleEnvelope(makeTestSubscribeEnvelope({ isHistory: true }))).toBe('history');
    const envelope = makeTestSubscribeEnvelope();
    expect(controller.handleEnvelope(envelope)).toBe('shown');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    expect(controller.handleEnvelope(makeTestGiftEnvelope())).toBe('ignored');
    expect(controller.handleEnvelope({ topic: 'live.event', data: 'garbage' })).toBe('ignored');
    expect(shown).toHaveLength(1);
  });
});
