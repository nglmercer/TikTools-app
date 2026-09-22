import { describe, expect, test } from 'bun:test';

import {
  ShareController,
  type ShareAlert,
} from './share-controller.ts';
import { createFakeClock } from './test-clock.ts';
import { makeTestGiftEnvelope, makeTestShareEnvelope } from './test-events.ts';

function setup(options: { visibleMs?: number; queueCapacity?: number } = {}) {
  const fake = createFakeClock();
  const shown: ShareAlert[] = [];
  const hidden: string[] = [];
  const controller = new ShareController(
    {
      onShow: (alert) => void shown.push(alert),
      onHide: (id) => void hidden.push(id),
    },
    { visibleMs: options.visibleMs ?? 4000, queueCapacity: options.queueCapacity, clock: fake.clock },
  );
  return { fake, shown, hidden, controller };
}

describe('share controller', () => {
  test('shows one alert, holds, then hides', () => {
    const { fake, shown, hidden, controller } = setup();
    expect(controller.handleEnvelope(makeTestShareEnvelope())).toBe('shown');
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
      makeTestShareEnvelope({ user: { avatarUrl: 'https://cdn.example/s.png' } }),
    );
    controller.handleEnvelope(makeTestShareEnvelope({ user: { nickname: 'No Avatar' } }));
    fake.advance(4000);
    expect(shown.map((alert) => alert.avatarUrl)).toEqual(['https://cdn.example/s.png', null]);
  });

  test('queues bursts and serves them in order', () => {
    const { fake, shown, hidden, controller } = setup();
    controller.handleEnvelope(makeTestShareEnvelope({ user: { nickname: 'First' } }));
    expect(controller.handleEnvelope(makeTestShareEnvelope({ user: { nickname: 'Second' } }))).toBe(
      'queued',
    );
    expect(controller.queueSize).toBe(1);
    fake.advance(4000);
    expect(hidden).toHaveLength(1);
    expect(shown.map((alert) => alert.displayName)).toEqual(['First', 'Second']);
    fake.advance(4000);
    expect(hidden).toHaveLength(2);
  });

  test('ignores history, duplicates, and non-share events', () => {
    const { shown, controller } = setup();
    expect(controller.handleEnvelope(makeTestShareEnvelope({ isHistory: true }))).toBe('history');
    const envelope = makeTestShareEnvelope();
    expect(controller.handleEnvelope(envelope)).toBe('shown');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    expect(controller.handleEnvelope(makeTestGiftEnvelope())).toBe('ignored');
    expect(controller.handleEnvelope({ topic: 'live.event', data: 'garbage' })).toBe('ignored');
    expect(shown).toHaveLength(1);
  });
});
