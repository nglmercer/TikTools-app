import { callbacks, installFrameControl, uninstallFrameControl, runFrames, createControl, createBackfillControl, expect, it, createLiveFeedState, type ControlClient } from './testing/live-feed-fixture.ts';

it('returning from a hidden feed replays retained native events instead of a marker', async () => {
  installFrameControl();
  const harness = createBackfillControl(() => ({
    oldestCursor: 9,
    latestCursor: 12,
    depth: 4,
    gap: false,
    truncated: false,
    events: [
      { cursor: 11, event: { kind: 'chat', author: 'a', text: 'recovered-11', msgId: 'r11' } },
      { cursor: 12, event: { kind: 'chat', author: 'a', text: 'recovered-12', msgId: 'r12' } },
    ],
  }));
  const live = createLiveFeedState(harness.control, callbacks('System'));
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'live-10', msgId: 'l10' }, cursor: 10 } as never);
  runFrames();
  live.setPresentationActive(false);
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'hidden-11', msgId: 'h11' }, cursor: 11 } as never);
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'hidden-12', msgId: 'h12' }, cursor: 12 } as never);
  runFrames();
  expect(live.events.value).toHaveLength(1);
  expect(live.droppedUiEvents.value).toBe(2);
  live.setPresentationActive(true);
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  expect(harness.calls).toEqual([{ method: 'ui.feed.backfill', params: { version: 1, after: 10 } }]);
  const texts = live.events.value.map((event) => event.text);
  expect(texts).toContain('live-10');
  expect(texts).toContain('recovered-11');
  expect(texts).toContain('recovered-12');
  expect(texts.some((text) => text?.includes('Feed resumed'))).toBe(false);
  expect(live.diagnostics().cursor).toBe(12);
  expect(live.droppedUiEvents.value).toBe(0);
  live.dispose();
  uninstallFrameControl();
});

it('a gap or truncation in the native replay still inserts a discontinuity marker', async () => {
  installFrameControl();
  const harness = createBackfillControl(() => ({
    oldestCursor: 9,
    latestCursor: 12,
    depth: 4,
    gap: true,
    truncated: false,
    events: [{ cursor: 11, event: { kind: 'chat', author: 'a', text: 'recovered-11', msgId: 'r11' } }],
  }));
  const live = createLiveFeedState(harness.control, callbacks('System'));
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'live-10', msgId: 'l10' }, cursor: 10 } as never);
  runFrames();
  live.setPresentationActive(false);
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'hidden-11', msgId: 'h11' }, cursor: 11 } as never);
  live.setPresentationActive(true);
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  const texts = live.events.value.map((event) => event.text);
  expect(texts).toContain('recovered-11');
  expect(texts.some((text) => text?.includes('Feed resumed'))).toBe(true);
  live.dispose();
  uninstallFrameControl();
});

it('a failed native replay degrades to the remaining discontinuity marker', async () => {
  installFrameControl();
  const harness = createBackfillControl(() => {
    throw new Error('host unavailable');
  });
  const live = createLiveFeedState(harness.control, callbacks('System'));
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'live-10', msgId: 'l10' }, cursor: 10 } as never);
  runFrames();
  live.setPresentationActive(false);
  harness.emit('live.ui-event', { event: { kind: 'chat', author: 'a', text: 'hidden-11', msgId: 'h11' }, cursor: 11 } as never);
  live.setPresentationActive(true);
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  expect(live.events.value.some((event) => event.text?.includes('Feed resumed'))).toBe(true);
  live.dispose();
  uninstallFrameControl();
});

it('deduplicates type and identity only, keeping identical gifts without usable ids', () => {
  installFrameControl();
  const harness = createControl();
  const live = createLiveFeedState(harness.control, callbacks('system'));
  for (const [kind, msgId] of [['chat', '42'], ['gift', '42'], ['gift', '43'], ['gift', '0'], ['gift', '0'], ['gift', ''], ['gift', ''], ['gift', undefined], ['gift', undefined]]) {
    harness.emit('live.ui-event', { event: { kind, author: 'alice', text: 'same', msgId } } as never);
  }
  runFrames();
  expect(live.events.value).toHaveLength(9);
  harness.emit('live.ui-event', { event: { kind: 'gift', author: 'alice', text: 'same', msgId: '42' } } as never);
  runFrames();
  expect(live.events.value).toHaveLength(9);
  expect(live.diagnostics().duplicates).toBe(1);
  live.dispose();
  uninstallFrameControl();
});

it('recovers from cursor zero, pages truncated backfill, and deduplicates realtime overlap by cursor', async () => {
  installFrameControl();
  const harness = createControl();
  const gift = { kind: 'gift' as const, author: 'alice', text: 'Rose', msgId: '0' };
  const calls: number[] = [];
  let answer: ((value: unknown) => void) | undefined;
  harness.control.call = (<T,>(_method: string, params?: { after?: number }): Promise<T> => {
    calls.push(params?.after ?? -1);
    if (calls.length === 1) return new Promise<T>((resolve) => { answer = (value) => resolve(value as T); });
    return Promise.resolve({ events: [{ cursor: 3, event: gift }], latestCursor: 3, truncated: false, gap: false } as T);
  }) as ControlClient['call'];
  const live = createLiveFeedState(harness.control, callbacks('system'));
  live.setPresentationActive(false);
  live.setPresentationActive(true);
  harness.emit('live.ui-event', { event: gift, cursor: 2 } as never);
  answer?.({ events: [{ cursor: 1, event: gift }, { cursor: 2, event: gift }], latestCursor: 3, truncated: true, gap: false });
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  expect(calls).toEqual([0, 2]);
  expect(live.events.value.map((event) => event.feedCursor)).toEqual([1, 2, 3]);
  expect(live.events.value.filter((event) => event.kind === 'gift')).toHaveLength(3);
  expect(live.diagnostics().cursor).toBe(3);
  expect(live.diagnostics().recoveryGaps).toBe(0);
  live.dispose();
  uninstallFrameControl();
});

it('recovers cursor gaps while visible and preserves distinct authoritative deliveries with the same msgId', async () => {
  installFrameControl();
  const event = { kind: 'gift', author: 'alice', text: 'Gift without artwork', msgId: '42' };
  const harness = createBackfillControl(() => ({ events: [{ cursor: 2, event }], latestCursor: 3, truncated: false, gap: false }));
  const live = createLiveFeedState(harness.control, callbacks('system'));
  harness.emit('live.ui-event', { event, cursor: 1 } as never);
  harness.emit('live.ui-event', { event, cursor: 3 } as never);
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  expect(harness.calls[0]).toEqual({ method: 'ui.feed.backfill', params: { version: 1, after: 1 } });
  expect(live.events.value.map((event) => event.feedCursor)).toEqual([1, 2, 3]);
  expect(live.diagnostics().recoveryGaps).toBe(0);
  live.dispose();
  uninstallFrameControl();
});

it('retains the earliest hole when another realtime gap arrives during backfill', async () => {
  installFrameControl();
  let release: ((value: unknown) => void) | undefined;
  let requests = 0;
  const event = (cursor: number) => ({ cursor, event: { kind: 'chat', author: 'a', text: String(cursor), msgId: '0' } });
  const harness = createBackfillControl(() => {
    requests += 1;
    if (requests === 1) return new Promise((resolve) => { release = resolve; });
    return { events: [event(4), event(5)], latestCursor: 5, gap: false, truncated: false };
  });
  const live = createLiveFeedState(harness.control, callbacks('system'));
  for (const cursor of [1, 3, 5]) harness.emit('live.ui-event', event(cursor) as never);
  release?.({ events: [event(2), event(3)], latestCursor: 3, gap: false, truncated: false });
  await new Promise((resolve) => setTimeout(resolve, 0));
  runFrames();
  expect(harness.calls).toHaveLength(2);
  expect(live.events.value.map((event) => event.feedCursor)).toEqual([1, 2, 3, 4, 5]);
  live.dispose();
  uninstallFrameControl();
});

it('folds a streakable combo sharing one groupId into a single live gift row', () => {
  installFrameControl();
  const harness = createControl();
  const live = createLiveFeedState(harness.control, callbacks('system'));
  const streak = (count: number, msgId: string, repeatEnd: boolean) => ({
    event: {
      kind: 'gift' as const,
      author: 'mango',
      text: `sent ${count}× Rose`,
      msgId,
      giftDetails: { giftId: '1', name: 'Rose', count, diamonds: count, groupId: 'g-9', repeatEnd, streakable: true },
    },
  });
  for (const [count, msgId, repeatEnd] of [[1, '101', false], [2, '102', false], [5, '103', true]] as const) {
    harness.emit('live.ui-event', streak(count, msgId, repeatEnd) as never);
  }
  runFrames();
  expect(live.events.value).toHaveLength(1);
  expect(live.events.value[0]?.giftDetails?.count).toBe(5);
  expect(live.events.value[0]?.giftDetails?.repeatEnd).toBe(true);
  live.dispose();
  uninstallFrameControl();
});

it('keeps gifts when a chat flood exceeds the visible buffer', () => {
  installFrameControl();
  const harness = createControl();
  const live = createLiveFeedState(harness.control, callbacks('system'));
  harness.emit('live.ui-event', { event: { kind: 'gift', author: 'keeper', text: 'Rose', msgId: 'g1', giftDetails: { giftId: '7', name: 'Rose', count: 1, diamonds: 1 } } } as never);
  for (let i = 0; i < 400; i += 1) {
    harness.emit('live.ui-event', { event: { kind: 'chat', author: `chatter-${i}`, text: `message ${i}`, msgId: `c${i}` } } as never);
  }
  runFrames();
  expect(live.events.value.some((event) => event.kind === 'gift' && event.author === 'keeper')).toBe(true);
  expect(live.events.value.filter((event) => event.kind === 'gift')).toHaveLength(1);
  live.dispose();
  uninstallFrameControl();
});
