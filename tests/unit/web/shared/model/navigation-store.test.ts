import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';
import { useNavigationStore } from '../../../../../src/web/shared/model/navigation-store.ts';

function setup() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return useNavigationStore();
}

beforeEach(() => {
  localStorage.clear();
});

describe('navigation store sidebar', () => {
  it('starts expanded and persists collapsing', () => {
    const store = setup();
    expect(store.sidebarCollapsed).toBe(false);
    store.toggleSidebar();
    expect(store.sidebarCollapsed).toBe(true);
  });

  it('restores the persisted collapsed state', () => {
    const first = setup();
    first.setSidebarCollapsed(true);
    const second = setup();
    expect(second.sidebarCollapsed).toBe(true);
  });
});
