import { callbacks, createControl, chatEvent, describe, expect, it, createLiveFeedState } from './testing/live-feed-fixture.ts';

describe('live feed rendering', () => {
  it('renders every event immediately, capped at 100 per category', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    for (let i = 0; i < 1000; i += 1) {
      harness.emit('live.ui-event', chatEvent('burst', `user-${i}`, i) as never);
    }
    expect(live.events.value).toHaveLength(100);
    expect(live.events.value[0]?.author).toBe('burst-user-900');
    expect(live.diagnostics().received).toBe(1000);
    live.dispose();
  });

  it('keeps recent events of every category when one kind floods', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', chatEvent('chat', 'alice', 0) as never);
    for (let i = 0; i < 200; i += 1) {
      harness.emit('live.ui-event', { event: { kind: 'like', author: 'fan', text: 'like', msgId: `like-${i}` } } as never);
    }
    expect(live.events.value.filter((event) => event.kind === 'chat')).toHaveLength(1);
    expect(live.events.value.filter((event) => event.kind === 'like')).toHaveLength(100);
    live.dispose();
  });

  it('paused auto-scroll counts each event as unread', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    live.handleToggleAutoScroll();
    expect(live.autoScroll.value).toBe(false);
    for (let i = 0; i < 10; i += 1) {
      harness.emit('live.ui-event', chatEvent('paused', `user-${i}`, i) as never);
    }
    expect(live.unreadCount.value).toBe(10);
    live.scrollToBottom();
    expect(live.unreadCount.value).toBe(10);
    live.dispose();
  });

  it('reconnect reset clears the feed', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    for (let i = 0; i < 5; i += 1) {
      harness.emit('live.ui-event', chatEvent('reset', `user-${i}`, i) as never);
    }
    live.resetEvents();
    expect(live.events.value).toHaveLength(0);
    expect(live.unreadCount.value).toBe(0);
    live.dispose();
  });

  it('dispose unsubscribes topics and ignores later events', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', chatEvent('dispose', 'user-0', 0) as never);
    live.dispose();
    harness.emit('live.ui-event', chatEvent('dispose', 'user-1', 1) as never);
    expect(live.events.value).toHaveLength(1);
    expect(harness.unsubscribed).toContain('live.ui-event');
    expect(harness.unsubscribed).toContain('room.stats');
  });

  it('duplicate replays are ignored', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    const payload = {
      event: { kind: 'chat' as const, author: 'alice', text: 'hello', msgId: 'replay-1' },
    };
    harness.emit('live.ui-event', payload as never);
    harness.emit('live.ui-event', payload as never);
    expect(live.events.value).toHaveLength(1);
    live.dispose();
  });

  it('does not expose skipped-update or coalescing diagnostics', () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('system'));
    harness.emit('live.ui-event', chatEvent('plain', 'alice', 1) as never);
    expect(live.diagnostics()).not.toHaveProperty('dropped');
    expect(live.diagnostics()).not.toHaveProperty('presentationCoalesced');
    expect(live).not.toHaveProperty('droppedUiEvents');
    live.dispose();
  });

  it('host projection stats trigger backfill without counting skipped UI updates', async () => {
    const harness = createControl();
    const live = createLiveFeedState(harness.control, callbacks('System'));
    controlStats(harness, { feedSuspended: 0, feedBusDropped: 0, feedForwarded: 0 });
    harness.emit('live.ui-event', { ...chatEvent('sequence', 'alice', 1), sequence: 1 } as never);
    controlStats(harness, { feedSuspended: 5, feedBusDropped: 3, feedForwarded: 4 });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(live.diagnostics()).not.toHaveProperty('dropped');
    expect(live.events.value.some((event) => event.text?.includes('Feed resumed'))).toBe(true);
    live.dispose();
  });
});

function controlStats(harness: ReturnType<typeof createControl>, data: Record<string, number>): void {
  harness.emit('ui.projection.stats', data as never);
}
