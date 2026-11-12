import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { nextTick, shallowRef } from 'vue';
import type { DisplayEvent } from '@/types.ts';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { createFeedViewport } from './feed-viewport.ts';

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
