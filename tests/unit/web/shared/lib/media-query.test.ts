import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { useMediaQuery } from './media-query.ts';

function stubMatchMedia(matches: boolean) {
  const listeners = new Set<(event: { matches: boolean }) => void>();
  const list = {
    matches,
    addEventListener: vi.fn((_type: string, listener: (event: { matches: boolean }) => void) => {
      listeners.add(listener);
    }),
    removeEventListener: vi.fn(((_type: string, listener: (event: { matches: boolean }) => void) => {
      listeners.delete(listener);
    }) as (type: string, listener: (event: { matches: boolean }) => void) => void),
    addListener: vi.fn(),
    removeListener: vi.fn(),
  };
  vi.stubGlobal('matchMedia', vi.fn(() => list));
  return { list, listeners };
}

function mountQuery(query: string) {
  const host = defineComponent({
    setup: () => ({ matches: useMediaQuery(query) }),
    render: () => null,
  });
  return mount(host);
}

describe('useMediaQuery', () => {
  it('reports no match when matchMedia is unavailable', () => {
    vi.stubGlobal('matchMedia', undefined);
    const wrapper = mountQuery('(width < 900px)');
    expect(wrapper.vm.matches).toBe(false);
    wrapper.unmount();
    vi.unstubAllGlobals();
  });

  it('tracks viewport changes and cleans up', () => {
    const { list, listeners } = stubMatchMedia(false);
    const wrapper = mountQuery('(width < 900px)');
    expect(wrapper.vm.matches).toBe(false);
    for (const listener of listeners) listener({ matches: true });
    expect(wrapper.vm.matches).toBe(true);
    wrapper.unmount();
    expect(list.removeEventListener).toHaveBeenCalledTimes(1);
    vi.unstubAllGlobals();
  });
});
