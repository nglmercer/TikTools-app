import { describe, expect, test } from 'bun:test';

import { giftCounts, GiftController, type GiftAlertView } from './gift-controller.ts';
import { createFakeClock } from './test-clock.ts';
import {
  makeTestFollowEnvelope,
  makeTestGiftCombo,
  makeTestGiftEnvelope,
} from './test-events.ts';
import type { GiftAutomationData } from './event-types.ts';

function giftData(overrides: Partial<GiftAutomationData> = {}): GiftAutomationData {
  return {
    giftId: 'gift-rose',
    giftName: 'Rose',
    diamondCount: 1,
    repeatCount: 1,
    comboCount: 1,
    groupId: 'group-1',
    repeatEnd: false,
    streakable: false,
    giftIconUrl: null,
    method: 'test',
    msgId: 'msg-1',
    isHistory: false,
    ...overrides,
  };
}

function setup(options: { visibleMs?: number; comboTimeoutMs?: number; minimumDiamonds?: number } = {}) {
  const fake = createFakeClock();
  const shown: GiftAlertView[] = [];
  const updated: GiftAlertView[] = [];
  const hidden: string[] = [];
  const controller = new GiftController(
    {
      onShow: (alert) => void shown.push({ ...alert }),
      onUpdate: (alert) => void updated.push({ ...alert }),
      onHide: (key) => void hidden.push(key),
    },
    {
      visibleMs: options.visibleMs ?? 5000,
      comboTimeoutMs: options.comboTimeoutMs ?? 3000,
      minimumDiamonds: options.minimumDiamonds ?? 0,
      clock: fake.clock,
    },
  );
  return { fake, shown, updated, hidden, controller };
}

describe('gift counts', () => {
  test('uses the max of repeat/combo counts', () => {
    expect(giftCounts(giftData({ repeatCount: 2, comboCount: 5, diamondCount: 10 }))).toEqual({
      count: 5,
      diamondsEach: 10,
      totalDiamonds: 50,
    });
  });

  test('floors zero counts and diamonds at one', () => {
    expect(giftCounts(giftData({ repeatCount: 0, comboCount: 0, diamondCount: 0 }))).toEqual({
      count: 1,
      diamondsEach: 1,
      totalDiamonds: 1,
    });
  });
});

describe('gift controller', () => {
  test('enqueues non-streakable gifts immediately', () => {
    const { fake, shown, hidden, controller } = setup();
    expect(controller.handleEnvelope(makeTestGiftEnvelope({ giftName: 'Galaxy' }))).toBe('shown');
    expect(shown).toHaveLength(1);
    expect(shown[0]).toMatchObject({ giftName: 'Galaxy', count: 1, totalDiamonds: 1000, streaking: false });
    fake.advance(5000);
    expect(hidden).toHaveLength(1);
  });

  test('aggregates a streak into one live alert', () => {
    const { fake, shown, updated, hidden, controller } = setup();
    const combo = makeTestGiftCombo(5, { repeatEnd: false, giftName: 'Rose', diamondCount: 1 });
    const results = combo.map((envelope) => controller.handleEnvelope(envelope));
    expect(results[0]).toBe('shown');
    expect(results.slice(1)).toEqual(['updated', 'updated', 'updated', 'updated']);
    expect(shown).toHaveLength(1);
    expect(updated.map((alert) => alert.count)).toEqual([2, 3, 4, 5]);
    expect(controller.activeCount).toBe(1);
    // The live streak stays visible while aggregating (no hide mid-streak).
    fake.advance(2000);
    expect(hidden).toHaveLength(0);
  });

  test('finalizes on repeatEnd and holds the completed alert', () => {
    const { fake, shown, updated, hidden, controller } = setup();
    for (const envelope of makeTestGiftCombo(3, { diamondCount: 10 })) {
      controller.handleEnvelope(envelope);
    }
    expect(shown).toHaveLength(1);
    const last = updated[updated.length - 1];
    expect(last).toMatchObject({ count: 3, totalDiamonds: 30, streaking: false });
    expect(hidden).toHaveLength(0);
    fake.advance(4999);
    expect(hidden).toHaveLength(0);
    fake.advance(1);
    expect(hidden).toHaveLength(1);
  });

  test('finalizes stale combos when repeatEnd never arrives', () => {
    const { fake, shown, updated, hidden, controller } = setup({ comboTimeoutMs: 2000 });
    controller.handleEnvelope(
      makeTestGiftEnvelope({ groupId: 'g1', streakable: true, repeatCount: 2, comboCount: 2 }),
    );
    expect(shown).toHaveLength(1);
    expect(shown[0]?.streaking).toBe(true);
    fake.advance(1999);
    expect(updated.filter((alert) => !alert.streaking)).toHaveLength(0);
    fake.advance(1);
    const finished = updated[updated.length - 1];
    expect(finished).toMatchObject({ count: 2, streaking: false });
    expect(controller.activeCount).toBe(0);
    fake.advance(5000);
    expect(hidden).toHaveLength(1);
  });

  test('tracks two simultaneous gift groups independently', () => {
    // Long combo timeout so the waiting group stays live, not stale.
    const { fake, shown, hidden, controller } = setup({ comboTimeoutMs: 30_000 });
    controller.handleEnvelope(
      makeTestGiftEnvelope({ id: 'a1', groupId: 'a', giftName: 'Rose', streakable: true, repeatCount: 1, comboCount: 1 }),
    );
    controller.handleEnvelope(
      makeTestGiftEnvelope({ id: 'b1', groupId: 'b', giftName: 'Galaxy', streakable: true, repeatCount: 1, comboCount: 1 }),
    );
    expect(controller.activeCount).toBe(2);
    expect(shown).toHaveLength(1);
    expect(shown[0]?.giftName).toBe('Rose');
    // Finishing the displayed streak reveals the waiting combo live.
    controller.handleEnvelope(
      makeTestGiftEnvelope({ id: 'a2', groupId: 'a', giftName: 'Rose', streakable: true, repeatCount: 2, comboCount: 2, repeatEnd: true }),
    );
    fake.advance(5000);
    const firstKey = shown[0]?.key;
    expect(firstKey).toBeDefined();
    expect(hidden).toEqual([firstKey as string]);
    expect(shown).toHaveLength(2);
    expect(shown[1]).toMatchObject({ giftName: 'Galaxy', streaking: true });
  });

  test('filters below-minimum gifts and ignores history/duplicates', () => {
    const { shown, controller } = setup({ minimumDiamonds: 100 });
    expect(controller.handleEnvelope(makeTestGiftEnvelope({ diamondCount: 1 }))).toBe('filtered');
    expect(controller.handleEnvelope(makeTestGiftEnvelope({ isHistory: true }))).toBe('history');
    const envelope = makeTestGiftEnvelope({ diamondCount: 500 });
    expect(controller.handleEnvelope(envelope)).toBe('shown');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    expect(controller.handleEnvelope(makeTestFollowEnvelope())).toBe('ignored');
    expect(shown).toHaveLength(1);
  });
});
