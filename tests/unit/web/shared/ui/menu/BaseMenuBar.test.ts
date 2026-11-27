import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import BaseMenuBar from '../../../../../../src/web/shared/ui/menu/BaseMenuBar.vue';
import type { MenuBarMenu } from '../../../../../../src/web/shared/ui/menu/menu-types.ts';

enableAutoUnmount(afterEach);

const menus: MenuBarMenu[] = [
  { id: 'file', label: 'File', items: [{ id: 'connect', label: 'Connect' }, { id: 'exit', label: 'Exit' }] },
  { id: 'view', label: 'View', items: [{ id: 'feed', label: 'Feed' }] },
];

function mountMenuBar() {
  return mount(BaseMenuBar, { attachTo: document.body, props: { menus } });
}

function openMenuLabels(): string[] {
  return Array.from(document.querySelectorAll('[role="menu"] [role="menuitem"]')).map((item) => item.textContent?.trim() ?? '');
}

function press(key: string): void {
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
}

describe('BaseMenuBar', () => {
  it('exposes a single tab stop and moves focus with arrow keys', async () => {
    const wrapper = mountMenuBar();
    const [file, view] = wrapper.findAll<HTMLElement>('[role="menuitem"]').map((item) => item.element);
    expect(file!.tabIndex).toBe(0);
    expect(view!.tabIndex).toBe(-1);
    file!.focus();
    press('ArrowRight');
    expect(document.activeElement).toBe(view);
  });

  it('opens with the keyboard and moves between menus while open', async () => {
    const wrapper = mountMenuBar();
    wrapper.findAll<HTMLElement>('[role="menuitem"]')[0]!.element.focus();
    press('ArrowDown');
    await nextTick();
    expect(openMenuLabels()).toEqual(['Connect', 'Exit']);
    expect(document.activeElement?.textContent).toContain('Connect');
    press('ArrowRight');
    await nextTick();
    await nextTick();
    expect(openMenuLabels()).toEqual(['Feed']);
  });

  it('emits the selected action and closes', async () => {
    const wrapper = mountMenuBar();
    await wrapper.findAll('[role="menuitem"]')[0]!.trigger('click');
    await nextTick();
    document.querySelectorAll<HTMLElement>('[role="menu"] [role="menuitem"]')[1]!.click();
    await nextTick();
    await nextTick();
    expect(wrapper.emitted('select')?.[0]?.[0]).toMatchObject({ id: 'exit' });
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });

  it('toggles a menu closed when its title is clicked again', async () => {
    const wrapper = mountMenuBar();
    const file = wrapper.findAll('[role="menuitem"]')[0]!;
    await file.trigger('click');
    expect(file.attributes('aria-expanded')).toBe('true');
    await file.trigger('click');
    expect(file.attributes('aria-expanded')).toBe('false');
  });
});
