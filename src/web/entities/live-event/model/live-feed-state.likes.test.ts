import { callbacks, installFrameControl, uninstallFrameControl, runFrames, createControl, chatEvent, likeEvent, beforeEach, describe, expect, it, vi, createLiveFeedState } from './testing/live-feed-fixture.ts';

describe('presentation-only per-viewer like bursts', () => {
  beforeEach(() => {
    installFrameControl();
    return () => uninstallFrameControl();
  });

  it('folds 15 + 15 + 5 into one displayed 35-like row but keeps the latest packet count', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 15, 'like-1') as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 30, 'like-2') as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 5, 35, 'like-3') as never);
    runFrames();
    expect(live.events.value).toHaveLength(1);
    expect(live.events.value[0]).toMatchObject({
      id: 0, author: 'lizbeth', likeCount: 5, likeUserTotal: 35, msgId: 'like-3',
      i18nParams: { count: 5 },
    });
    expect(live.diagnostics().received).toBe(3);
    expect(live.diagnostics().presentationCoalesced).toBe(2);
    live.dispose();
  });

  it('moves an existing committed like row after intervening chat without creating a duplicate', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 15, 'like-1') as never);
    runFrames();
    const likeRowId = live.events.value[0]!.id;
    harness.emit('live.ui-event', chatEvent('chat', 'other', 1) as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 30, 'like-2') as never);
    runFrames();
    expect(live.events.value.map((row) => row.kind)).toEqual(['chat', 'like']);
    expect(live.events.value[1]).toMatchObject({
      id: likeRowId, author: 'lizbeth', likeCount: 15, likeUserTotal: 30,
    });
    live.dispose();
  });

  it('does not group separate viewers, rooms, or events with no trustworthy identity', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', likeEvent('a', 15, 15, 'a1') as never);
    harness.emit('live.ui-event', likeEvent('b', 15, 15, 'b1') as never);
    harness.emit('live.ui-event', likeEvent('a', 15, 30, 'a2', 'another-room') as never);
    harness.emit('live.ui-event', likeEvent('viewer', 15, 15, 'missing-1', 'same-room', false) as never);
    harness.emit('live.ui-event', likeEvent('viewer', 15, 30, 'missing-2', 'same-room', false) as never);
    runFrames();
    expect(live.events.value).toHaveLength(5);
    expect(live.events.value.map((row) => row.likeUserTotal)).toEqual([15, 15, 30, 15, 30]);
    live.dispose();
  });

  it('does not merge an older total, a replayed msgId, or likes after a 3-second pause', () => {
    const now = vi.spyOn(Date, 'now').mockReturnValue(1_000);
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    try {
      const first = likeEvent('lizbeth', 15, 15, 'like-1');
      harness.emit('live.ui-event', first as never);
      harness.emit('live.ui-event', first as never);
      runFrames();
      expect(live.events.value).toHaveLength(1);
      expect(live.diagnostics().duplicates).toBe(1);

      now.mockReturnValue(1_250);
      harness.emit('live.ui-event', likeEvent('lizbeth', 5, 10, 'late-out-of-order') as never);
      runFrames();
      expect(live.events.value).toHaveLength(2);

      now.mockReturnValue(4_251);
      harness.emit('live.ui-event', likeEvent('lizbeth', 20, 35, 'after-window') as never);
      runFrames();
      expect(live.events.value).toHaveLength(3);
      expect(live.events.value[2]?.likeUserTotal).toBe(35);
    } finally {
      live.dispose();
      now.mockRestore();
    }
  });
});
