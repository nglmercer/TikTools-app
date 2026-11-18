import { callbacks, installFrameControl, uninstallFrameControl, runFrames, createControl, chatEvent, beforeEach, describe, expect, it, vi, createLiveFeedState } from './testing/live-feed-fixture.ts';

describe('live feed batching', () => {
  beforeEach(() => {
    installFrameControl();
    return () => uninstallFrameControl();
  });

  it('a 1,000-event burst renders one frame, capped at 300 visible', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    for (let i = 0; i < 1000; i += 1) {
      harness.emit('live.ui-event', chatEvent('burst', `user-${i}`, i) as never);
    }
    expect(live.events.value).toHaveLength(0);
    runFrames(1);
    expect(live.events.value).toHaveLength(300);
    expect(live.events.value[0]?.author).toBe('burst-user-700');
    expect(live.droppedUiEvents.value).toBe(100);
    live.dispose();
  });

  it('paused auto-scroll never mutates scroll and counts unread in bulk', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    live.handleToggleAutoScroll();
    expect(live.autoScroll.value).toBe(false);
    for (let i = 0; i < 10; i += 1) {
      harness.emit('live.ui-event', chatEvent('paused', `user-${i}`, i) as never);
    }
    runFrames(2);
    expect(live.unreadCount.value).toBe(10);
    live.scrollToBottom();
    runFrames(2);
    expect(live.unreadCount.value).toBe(10);
    live.dispose();
  });

  it('reconnect reset clears feed, staging, and counters', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    for (let i = 0; i < 5; i += 1) {
      harness.emit('live.ui-event', chatEvent('reset', `user-${i}`, i) as never);
    }
    live.resetEvents();
    runFrames(2);
    expect(live.events.value).toHaveLength(0);
    expect(live.unreadCount.value).toBe(0);
    live.dispose();
  });

  it('dispose cancels frames and unsubscribes topics', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', chatEvent('dispose', 'user-0', 0) as never);
    live.dispose();
    runFrames(3);
    expect(live.events.value).toHaveLength(0);
    harness.emit('live.ui-event', chatEvent('dispose', 'user-1', 1) as never);
    runFrames(3);
    expect(live.events.value).toHaveLength(0);
    expect(harness.unsubscribed).toContain('live.ui-event');
    expect(harness.unsubscribed).toContain('room.stats');
  });

  it('duplicate replays never reach the staged buffer', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    const payload = {
      event: { kind: 'chat' as const, author: 'alice', text: 'hello', msgId: 'replay-1' },
    };
    harness.emit('live.ui-event', payload as never);
    harness.emit('live.ui-event', payload as never);
    runFrames(2);
    expect(live.events.value).toHaveLength(1);
    live.dispose();
  });
});

it('feed identity caches belong to an instance and preserve only the same room', () => {
  installFrameControl();
  const harness = createControl();
  const live = createLiveFeedState(harness.control, callbacks('system'));
  const event = { event: { kind: 'chat', author: 'viewer', text: 'hello', msgId: 'stable' } };
  harness.emit('live.connected', { uniqueId: 'a', roomId: '1' } as never);
  harness.emit('live.ui-event', event as never);
  runFrames();
  live.resetEvents();
  harness.emit('live.connected', { uniqueId: 'a', roomId: '1' } as never);
  harness.emit('live.ui-event', event as never);
  runFrames();
  expect(live.events.value).toHaveLength(0);
  harness.emit('live.connected', { uniqueId: 'b', roomId: '2' } as never);
  harness.emit('live.ui-event', event as never);
  runFrames();
  expect(live.events.value).toHaveLength(1);
  live.dispose();
  const next = createLiveFeedState(harness.control, callbacks('system'));
  harness.emit('live.ui-event', event as never);
  runFrames();
  expect(next.events.value).toHaveLength(1);
  next.dispose();
  uninstallFrameControl();
});

it('message IDs expire after the replay age limit', () => {
  installFrameControl();
  const clock = vi.spyOn(Date, 'now').mockReturnValue(0);
  const harness = createControl();
  const live = createLiveFeedState(harness.control, callbacks('system'));
  try {
    const event = { event: { kind: 'chat', author: 'viewer', text: 'hello', msgId: 'aged' } };
    harness.emit('live.ui-event', event as never);
    runFrames();
    live.resetEvents();
    harness.emit('live.ui-event', event as never);
    runFrames();
    expect(live.events.value).toHaveLength(0);
    clock.mockReturnValue(10 * 60_000 + 1);
    harness.emit('live.ui-event', event as never);
    runFrames();
    expect(live.events.value).toHaveLength(1);
  } finally {
    live.dispose();
    clock.mockRestore();
    uninstallFrameControl();
  }
});

it('hidden feed preserves accepted staging and returns with a bounded discontinuity', () => {
  installFrameControl();
  const control = createControl();
  const live = createLiveFeedState(control.control, callbacks('System'));
  const send = (id: number) => control.handlers.get('live.ui-event')?.forEach((handler) =>
    handler({ event: { kind: 'chat', author: 'alice', text: String(id), msgId: String(id) } } as never));
  send(0);
  live.setPresentationActive(false);
  const before = live.events.value;
  for (let i = 1; i <= 10_000; i += 1) send(i);
  runFrames();
  expect(live.events.value).toBe(before);
  expect(live.diagnostics().staged).toBe(0);
  expect(live.diagnostics().commits).toBe(1);
  expect(live.droppedUiEvents.value).toBe(10_000);
  live.setPresentationActive(true);
  send(10_001);
  runFrames();
  expect(live.events.value).toHaveLength(3);
  expect(live.events.value[0]?.text).toBe('0');
  expect(live.events.value[1]?.text).toContain('Feed resumed');
  live.dispose();
  uninstallFrameControl();
});

it('a stalled native frame clock commits via one bounded timeout and disposal cancels it', async () => {
  installFrameControl();
  const control = createControl();
  const live = createLiveFeedState(control.control, callbacks('System'));
  control.emit('live.ui-event', chatEvent('stalled-clock', 'alice', 1) as never);
  expect(live.events.value).toHaveLength(0);
  await new Promise((resolve) => setTimeout(resolve, 120));
  expect(live.events.value).toHaveLength(1);
  expect(live.diagnostics().commits).toBe(1);
  runFrames();
  expect(live.diagnostics().commits).toBe(1);
  control.emit('live.ui-event', chatEvent('stalled-clock', 'alice', 2) as never);
  live.dispose();
  await new Promise((resolve) => setTimeout(resolve, 120));
  expect(live.diagnostics().commits).toBe(1);
  expect(live.diagnostics().timers).toBe(0);
  uninstallFrameControl();
});

it('feed counts source-bus and transport shedding separately without counting suspension twice', () => {
  installFrameControl();
  const control = createControl();
  const live = createLiveFeedState(control.control, callbacks('System'));
  control.emit('ui.projection.stats', { feedSuspended: 0, feedBusDropped: 0, feedForwarded: 0 } as never);
  control.emit('live.ui-event', { ...chatEvent('sequence', 'alice', 1), sequence: 1 } as never);
  control.emit('live.ui-event', { ...chatEvent('sequence', 'alice', 2), sequence: 4 } as never);
  control.emit('ui.projection.stats', { feedSuspended: 5, feedBusDropped: 3, feedForwarded: 4 } as never);
  expect(live.droppedUiEvents.value).toBe(10);
  control.emit('ui.projection.stats', { feedSuspended: 5, feedBusDropped: 3, feedForwarded: 4 } as never);
  expect(live.droppedUiEvents.value).toBe(10);
  live.dispose();
  uninstallFrameControl();
});
