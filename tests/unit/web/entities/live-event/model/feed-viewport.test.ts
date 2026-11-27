import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { defineComponent, h, nextTick, shallowRef } from 'vue';
import type { DisplayEvent } from '@/types.ts';
import { mount } from '@vue/test-utils';
import { toElement } from '../../../../support/dom.ts';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { createFeedViewport } from '../../../../../../src/web/entities/live-event/model/feed-viewport.ts';

const frames = new Map<number, () => void>();
let frameId = 0;
function flushFrames(): void {
  const pending = [...frames.values()];
  frames.clear();
  for (const callback of pending) callback();
}
function container(height: number): HTMLDivElement {
  const element = document.createElement('div');
  Object.defineProperty(element, 'scrollHeight', { value: height, configurable: true });
  return element;
}
beforeEach(() => {
  localStorage.clear();
  frames.clear();
  vi.useFakeTimers();
  vi.stubGlobal('requestAnimationFrame', (callback: () => void) => {
    frames.set(++frameId, callback);
    return frameId;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it('restores a saved reading position once enough feed content has arrived', async () => {
  saveViewState('feed', { autoScroll: false, scrollTop: 420 });
  const events = shallowRef<DisplayEvent[]>([]);
  const viewport = createFeedViewport(events, () => true);
  try {
    const element = container(100);
    viewport.setStreamContainerRef(element);
    flushFrames();
    expect(element.scrollTop).toBe(0);
    Object.defineProperty(element, 'scrollHeight', { value: 900 });
    events.value = [{ id: 1, kind: 'chat', author: 'alice', text: 'hello', receivedAt: 1 }];
    await nextTick();
    expect(element.scrollTop).toBe(420);
    element.scrollTop = 200;
    events.value = [...events.value];
    await nextTick();
    expect(element.scrollTop).toBe(200);
  } finally {
    viewport.dispose();
  }
});

it('retains the paused position across container replacement and stops persisting after disposal', () => {
  saveViewState('feed', { autoScroll: false });
  const viewport = createFeedViewport(shallowRef<DisplayEvent[]>([]), () => true);
  try {
    const first = container(900);
    viewport.setStreamContainerRef(first);
    flushFrames();
    first.scrollTop = 320;
    viewport.pause();
    expect(loadViewState().feed?.scrollTop).toBe(320);
    const replacement = container(1000);
    viewport.setStreamContainerRef(replacement);
    flushFrames();
    expect(replacement.scrollTop).toBe(320);
    viewport.dispose();
    replacement.scrollTop = 600;
    replacement.dispatchEvent(new Event('scroll'));
    viewport.scrollToBottom();
    vi.advanceTimersByTime(500);
    flushFrames();
    expect(loadViewState().feed?.scrollTop).toBe(320);
    expect(replacement.scrollTop).toBe(600);
    expect(frames.size).toBe(0);
  } finally {
    viewport.dispose();
  }
});

it('keeps the paused reading position anchored when eviction is hidden by appends', async () => {
  saveViewState('feed', { autoScroll: false });
  const row = (id: number): DisplayEvent => ({ id, kind: 'chat', author: `user${id}`, text: `message ${id}`, receivedAt: id });
  // Arrange: ten 28px rows, with the reader paused partway down the feed
  const events = shallowRef<DisplayEvent[]>(Array.from({ length: 10 }, (_, index) => row(index + 1)));
  const domRect = (top: number, height = 0): DOMRect => new DOMRect(0, top, 0, height);
  const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    if (this.classList.contains('feed-stream-test')) return domRect(0, 100);
    if (this.dataset.eventId) {
      const parent = this.parentElement;
      const index = parent ? Array.from(parent.children).indexOf(this) : 0;
      const scrollTop = parent instanceof HTMLElement ? parent.scrollTop : 0;
      const top = index * 28 - scrollTop;
      return domRect(top, 28);
    }
    return domRect(0);
  });
  let viewport!: ReturnType<typeof createFeedViewport>;
  const wrapper = mount(defineComponent({
    setup() {
      viewport = createFeedViewport(events, () => true);
      return () => h('div', {
        class: 'feed-stream-test',
        ref: (element) => viewport.setStreamContainerRef(element instanceof Element ? element : null),
      }, events.value.map((event) => h('div', { key: event.id, 'data-event-id': event.id })));
    },
  }), { attachTo: document.body });
  try {
    const element = toElement(wrapper.element, HTMLElement);
    Object.defineProperty(element, 'scrollHeight', {
      configurable: true,
      get: () => element.childElementCount * 28,
    });
    element.scrollTop = 100;

    // Act: evict the oldest row while appending two; the anchored row keeps its place on screen
    events.value = [...events.value.slice(1), row(11), row(12)];
    await nextTick();

    // Assert: scrollTop falls by exactly one row height, 100 to 72, so the reader sees no jump
    expect(element.scrollTop).toBe(72);
  } finally {
    wrapper.unmount();
    viewport.dispose();
    rectSpy.mockRestore();
  }
});

// A reader paused deep in a long feed must keep their row when the list is trimmed from both ends.
it('restores a paused anchor from the clamped scroll position', async () => {
  saveViewState('feed', { autoScroll: false });
  const row = (id: number): DisplayEvent => ({ id, kind: 'chat', author: `user${id}`, text: `message ${id}`, receivedAt: id });
  const events = shallowRef<DisplayEvent[]>(Array.from({ length: 100 }, (_, index) => row(index + 1)));
  const domRect = (top: number, height = 0): DOMRect => new DOMRect(0, top, 0, height);
  const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    if (this.classList.contains('feed-stream-test')) return domRect(0, 100);
    if (this.dataset.eventId) {
      const parent = this.parentElement;
      const index = parent ? Array.from(parent.children).indexOf(this) : 0;
      const scrollTop = parent instanceof HTMLElement ? parent.scrollTop : 0;
      const top = index * 10 - scrollTop;
      return domRect(top, 10);
    }
    return domRect(0);
  });
  let viewport!: ReturnType<typeof createFeedViewport>;
  const wrapper = mount(defineComponent({
    setup() {
      viewport = createFeedViewport(events, () => true);
      return () => h('div', {
        class: 'feed-stream-test',
        ref: (element) => viewport.setStreamContainerRef(element instanceof Element ? element : null),
      }, events.value.map((event) => h('div', { key: event.id, 'data-event-id': event.id })));
    },
  }), { attachTo: document.body });
  try {
    const element = toElement(wrapper.element, HTMLElement);
    Object.defineProperty(element, 'scrollHeight', {
      configurable: true,
      get: () => element.childElementCount * 10,
    });
    // Arrange: scrollTop reads back clamped to the current content height, as a browser does
    let assignedScrollTop = 0;
    Object.defineProperty(element, 'scrollTop', {
      configurable: true,
      get: () => Math.min(assignedScrollTop, Math.max(0, element.childElementCount * 10 - 100)),
      set: (value) => { assignedScrollTop = value; },
    });
    // Act: the reader pauses 500px down the feed, so the viewport anchors on the row at that offset
    element.scrollTop = 500;

    // Act: old rows are evicted from both ends, leaving 45 rows, so the anchored row changes index
    events.value = events.value.slice(45, 90);
    await nextTick();

    // Assert: the reader stays on its row at offset 50, not the stale offset, which would clamp to 350
    expect(element.scrollTop).toBe(50);
  } finally {
    wrapper.unmount();
    viewport.dispose();
    rectSpy.mockRestore();
  }
});
