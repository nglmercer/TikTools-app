import { mount } from '@vue/test-utils';
import { defineComponent, h, ref } from 'vue';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useShellInteractions } from './use-shell-interactions.ts';

const mediaListeners = new Set<(event: { matches: boolean }) => void>();
beforeEach(() => {
  mediaListeners.clear();
  vi.stubGlobal('matchMedia', () => ({
    matches: true,
    addEventListener: (_type: string, callback: (event: { matches: boolean }) => void) => mediaListeners.add(callback),
    removeEventListener: (_type: string, callback: (event: { matches: boolean }) => void) => mediaListeners.delete(callback),
  }));
});
afterEach(() => vi.unstubAllGlobals());
function mountInteractions() {
  const desktopToggle = vi.fn();
  let interactions!: ReturnType<typeof useShellInteractions>;
  const wrapper = mount(defineComponent({ setup() {
    const state = useShellInteractions(ref(true), desktopToggle);
    interactions = state;
    return () => h('main', { ref: state.contentRef });
  } }));
  return { wrapper, interactions, desktopToggle };
}
function resize(matches: boolean): void {
  for (const callback of mediaListeners) callback({ matches });
}
function swipeLeft(): void {
  for (const [type, x] of [['touchstart', 220], ['touchend', 80]] as const) {
    const event = new Event(type);
    Object.defineProperty(event, 'touches', { value: type === 'touchend' ? [] : [{ clientX: x, clientY: 100 }] });
    Object.defineProperty(event, 'changedTouches', { value: [{ clientX: x, clientY: 100 }] });
    window.dispatchEvent(event);
  }
}

it('respects consumed Escape and changes from a drawer to the desktop sidebar on resize', () => {
  const { wrapper, interactions: state, desktopToggle } = mountInteractions();
  try {
    state.toggleSidebar();
    const consumed = new KeyboardEvent('keydown', { key: 'Escape', cancelable: true });
    consumed.preventDefault();
    window.dispatchEvent(consumed);
    expect(state.drawerOpen.value).toBe(true);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(state.drawerOpen.value).toBe(false);
    state.toggleSidebar();
    resize(false);
    expect(state.drawerOpen.value).toBe(false);
    expect(state.sidebarRail.value).toBe(true);
    state.toggleSidebar();
    expect(desktopToggle).toHaveBeenCalledOnce();
  } finally {
    wrapper.unmount();
  }
});

it('remounts without leaving media, keyboard or swipe handlers behind', () => {
  for (let i = 0; i < 5; i++) {
    const { wrapper, interactions: state } = mountInteractions();
    expect(mediaListeners.size).toBe(1);
    state.toggleSidebar();
    swipeLeft();
    expect(state.drawerOpen.value).toBe(false);
    state.toggleSidebar();
    wrapper.unmount();
    expect(mediaListeners.size).toBe(0);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    swipeLeft();
    resize(false);
    expect(state.drawerOpen.value).toBe(true);
    expect(state.isNarrow.value).toBe(true);
  }
});
