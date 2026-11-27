import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { defineComponent, h, nextTick, ref } from 'vue';
import { toElement } from '../../../../support/dom.ts';
import BaseContextMenu from '../../../../../../src/web/shared/ui/menu/BaseContextMenu.vue';
import BaseMenu from '../../../../../../src/web/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '../../../../../../src/web/shared/ui/menu/menu-types.ts';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);

const items: MenuEntry[] = [
  { id: 'add', label: 'Add', shortcut: 'Ctrl+N' },
  { id: 'paste', label: 'Paste', disabled: true },
  { type: 'separator', id: 'sep' },
  {
    id: 'source',
    label: 'Add source',
    children: [
      { id: 'image', label: 'Image' },
      { id: 'text', label: 'Text' },
    ],
  },
  { id: 'lock', label: 'Lock', checked: true },
];

function mountMenu(menuItems: readonly MenuEntry[] = items) {
  const selected: MenuAction[] = [];
  const open = ref(false);
  const trigger = document.createElement('button');
  document.body.append(trigger);
  const wrapper = mount(
    defineComponent(() => () =>
      h(BaseMenu, {
        items: menuItems,
        anchor: trigger,
        open: open.value,
        focusFirst: true,
        'onUpdate:open': (value: boolean) => (open.value = value),
        onSelect: (action: MenuAction) => selected.push(action),
      }),
    ),
    { attachTo: document.body },
  );
  return { wrapper, open, selected, trigger };
}

function menus(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[role="menu"]'));
}

function press(key: string): void {
  (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
}

describe('BaseMenu', () => {
  it('renders actions, separators, shortcuts and check state', async () => {
    const { open } = mountMenu();
    open.value = true;
    await nextTick();
    const menu = defined(menus()[0]);
    expect(menu.querySelectorAll('[role="separator"]')).toHaveLength(1);
    expect(menu.textContent).toContain('Ctrl+N');
    expect(menu.querySelector('[role="menuitemcheckbox"]')?.getAttribute('aria-checked')).toBe('true');
    expect(document.activeElement?.textContent).toContain('Add');
  });

  it('skips disabled items and separators with the arrow keys', async () => {
    const { open } = mountMenu();
    open.value = true;
    await nextTick();
    press('ArrowDown');
    expect(document.activeElement?.textContent).toContain('Add source');
  });

  it('opens a submenu with ArrowRight and closes it with ArrowLeft', async () => {
    const { open } = mountMenu();
    open.value = true;
    await nextTick();
    press('ArrowDown');
    press('ArrowRight');
    await nextTick();
    await nextTick();
    expect(menus()).toHaveLength(2);
    expect(document.activeElement?.textContent).toContain('Image');
    press('ArrowLeft');
    await nextTick();
    expect(menus()).toHaveLength(1);
    expect(document.activeElement?.textContent).toContain('Add source');
  });

  it('selects nested actions, closes and restores focus', async () => {
    const { open, selected, trigger } = mountMenu();
    trigger.focus();
    open.value = true;
    await nextTick();
    press('ArrowDown');
    press('ArrowRight');
    await nextTick();
    await nextTick();
    press('ArrowDown');
    press('Enter');
    await nextTick();
    expect(selected.map((action) => action.id)).toEqual(['text']);
    expect(open.value).toBe(false);
    expect(menus()).toHaveLength(0);
    expect(document.activeElement).toBe(trigger);
  });

  it('closes on Escape and on outside pointer down', async () => {
    const { open } = mountMenu();
    open.value = true;
    await nextTick();
    press('Escape');
    expect(open.value).toBe(false);
    open.value = true;
    await nextTick();
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
    expect(open.value).toBe(false);
  });

  it('renders every entry when the menu has many items', async () => {
    const many: MenuEntry[] = Array.from({ length: 40 }, (_, index) => ({ id: `item-${index}`, label: `Item ${index}` }));
    const { open, selected } = mountMenu(many);
    open.value = true;
    await nextTick();
    const rendered = defined(menus()[0]).querySelectorAll('[role="menuitem"]');
    expect(rendered).toHaveLength(many.length);
    toElement(rendered[rendered.length - 1], HTMLElement).click();
    expect(selected.map((action) => action.id)).toEqual(['item-39']);
  });

  it('shows the full text of a long label instead of clipping it', async () => {
    const label = 'Revisar autoridad guardada y ejecución delegada con un nombre de biblioteca extremadamente largo que no cabe en una sola línea';
    const { open } = mountMenu([{ id: 'long', label }]);
    open.value = true;
    await nextTick();
    expect(defined(menus()[0]).querySelector('.menu-list__label')?.textContent).toBe(label);
  });
});

describe('BaseContextMenu', () => {
  it('opens at the pointer on contextmenu and reports the trigger event', async () => {
    const wrapper = mount(BaseContextMenu, {
      attachTo: document.body,
      props: { items },
      slots: { default: '<div class="target">Sources</div>' },
    });
    const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 40, clientY: 60 });
    wrapper.get('.target').element.dispatchEvent(event);
    await nextTick();
    expect(event.defaultPrevented).toBe(true);
    expect(wrapper.emitted('open')).toHaveLength(1);
    expect(menus()).toHaveLength(1);
  });

  it('keeps the native menu when disabled', async () => {
    const wrapper = mount(BaseContextMenu, {
      attachTo: document.body,
      props: { items, disabled: true },
      slots: { default: '<div class="target">Sources</div>' },
    });
    const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    wrapper.get('.target').element.dispatchEvent(event);
    await nextTick();
    expect(event.defaultPrevented).toBe(false);
    expect(menus()).toHaveLength(0);
  });
});
