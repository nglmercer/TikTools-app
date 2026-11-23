import { callbacks, createControl, chatEvent, likeEvent, describe, expect, it, vi, createLiveFeedState } from './testing/live-feed-fixture.ts';

describe('like events in the feed', () => {
  it('keeps every like packet as its own row with its own total', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 15, 'like-1') as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 30, 'like-2') as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 5, 35, 'like-3') as never);
    expect(live.events.value).toHaveLength(3);
    expect(live.events.value.map((row) => row.likeUserTotal)).toEqual([15, 30, 35]);
    expect(live.events.value[2]).toMatchObject({
      author: 'lizbeth', likeCount: 5, likeUserTotal: 35, msgId: 'like-3',
      i18nParams: { count: 5 },
    });
    expect(live.diagnostics().received).toBe(3);
    live.dispose();
  });

  it('appends new rows after intervening chat without moving committed rows', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 15, 'like-1') as never);
    const likeRowId = live.events.value[0]!.id;
    harness.emit('live.ui-event', chatEvent('chat', 'other', 1) as never);
    harness.emit('live.ui-event', likeEvent('lizbeth', 15, 30, 'like-2') as never);
    expect(live.events.value.map((row) => row.kind)).toEqual(['like', 'chat', 'like']);
    expect(live.events.value[0]?.id).toBe(likeRowId);
    expect(live.events.value[2]).toMatchObject({
      author: 'lizbeth', likeCount: 15, likeUserTotal: 30,
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
    expect(live.events.value).toHaveLength(5);
    expect(live.events.value.map((row) => row.likeUserTotal)).toEqual([15, 15, 30, 15, 30]);
    live.dispose();
  });

  it('still deduplicates a replayed message id', () => {
    vi.spyOn(Date, 'now').mockReturnValue(1_000);
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    try {
      const first = likeEvent('lizbeth', 15, 15, 'like-1');
      harness.emit('live.ui-event', first as never);
      harness.emit('live.ui-event', first as never);
      expect(live.events.value).toHaveLength(1);
      expect(live.diagnostics().duplicates).toBe(1);
    } finally {
      live.dispose();
      vi.restoreAllMocks();
    }
  });
});
