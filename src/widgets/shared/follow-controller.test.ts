import { describe, expect, test } from 'bun:test';

import {
  FollowController,
  type FollowAlert,
} from './follow-controller.ts';
import { createFakeClock } from './test-clock.ts';
import { makeTestFollowEnvelope, makeTestGiftEnvelope } from './test-events.ts';

function setup(options: { visibleMs?: number; queueCapacity?: number } = {}) {
  const fake = createFakeClock();
  const shown: FollowAlert[] = [];
  const hidden: string[] = [];
  const controller = new FollowController(
    {
      onShow: (alert) => void shown.push(alert),
      onHide: (id) => void hidden.push(id),
    },
    { visibleMs: options.visibleMs ?? 4000, queueCapacity: options.queueCapacity, clock: fake.clock },
  );
  return { fake, shown, hidden, controller };
}

describe('follow controller', () => {
  test('shows one alert, holds, then hides', () => {
    const { fake, shown, hidden, controller } = setup();
    expect(controller.handleEnvelope(makeTestFollowEnvelope())).toBe('shown');
    expect(shown).toHaveLength(1);
    const first = shown[0];
    if (first === undefined) throw new Error('expected a shown alert');
    expect(first.displayName).toBe('Viewer Name');
    expect(controller.currentAlert?.id).toBe(first.id);
    fake.advance(3999);
    expect(hidden).toHaveLength(0);
    fake.advance(1);
    expect(hidden).toEqual([first.id]);
    expect(controller.currentAlert).toBeNull();
  });

  test('carries the sender avatar, null when TikTok sent none', () => {
    const { fake, shown, controller } = setup();
    controller.handleEnvelope(
      makeTestFollowEnvelope({ user: { avatarUrl: 'https://cdn.example/a.png' } }),
    );
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'No Avatar' } }));
    fake.advance(4000);
    expect(shown.map((alert) => alert.avatarUrl)).toEqual(['https://cdn.example/a.png', null]);
  });

  test('queues bursts and serves them in order', () => {
    const { fake, shown, hidden, controller } = setup();
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'First' } }));
    expect(controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'Second' } }))).toBe(
      'queued',
    );
    expect(controller.queueSize).toBe(1);
    fake.advance(4000);
    expect(hidden).toHaveLength(1);
    expect(shown.map((alert) => alert.displayName)).toEqual(['First', 'Second']);
    fake.advance(4000);
    expect(hidden).toHaveLength(2);
  });

  test('ignores history, duplicates, and non-follow events', () => {
    const { shown, controller } = setup();
    expect(controller.handleEnvelope(makeTestFollowEnvelope({ isHistory: true }))).toBe('history');
    const envelope = makeTestFollowEnvelope();
    expect(controller.handleEnvelope(envelope)).toBe('shown');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    expect(controller.handleEnvelope(makeTestGiftEnvelope())).toBe('ignored');
    expect(controller.handleEnvelope({ topic: 'live.event', data: 'garbage' })).toBe('ignored');
    expect(shown).toHaveLength(1);
  });

  test('bounds the queue during bursts', () => {
    const { fake, shown, controller } = setup({ queueCapacity: 2 });
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'A' } }));
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'B' } }));
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'C' } }));
    controller.handleEnvelope(makeTestFollowEnvelope({ user: { nickname: 'D' } }));
    expect(controller.queueSize).toBe(2);
    fake.advance(12000);
    // Drop-oldest: A shows, then the freshest queued alerts C and D.
    expect(shown.map((alert) => alert.displayName)).toEqual(['A', 'C', 'D']);
  });

  test('falls back to uniqueId and Viewer for display names', () => {
    const { shown, controller } = setup();
    controller.handleEnvelope(
      makeTestFollowEnvelope({ id: 'f1', user: { nickname: '', uniqueId: 'handle' } }),
    );
    expect(shown[0]?.displayName).toBe('handle');
    expect(shown[0]?.uniqueId).toBe('@handle');
  });
});
