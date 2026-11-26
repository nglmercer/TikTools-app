import type { Component, InjectionKey } from 'vue';

export interface MenuAction {
  type?: 'action';
  id: string;
  label: string;
  icon?: Component;
  /** Tints the item icon — for pickers whose options double as a color legend. */
  color?: string;
  shortcut?: string;
  /** Secondary line rendered under the label in muted mono — for long
   *  technical values (variable paths, identifiers) that must never compete
   *  with the label for horizontal space the way a shortcut does. */
  detail?: string;
  disabled?: boolean;
  checked?: boolean;
  children?: readonly MenuEntry[];
}

export interface MenuSeparator {
  type: 'separator';
  id: string;
}

export type MenuEntry = MenuAction | MenuSeparator;

export function isMenuAction(entry: MenuEntry): entry is MenuAction {
  return entry.type !== 'separator';
}

export interface MenuContext {
  rootId: string;
  select: (action: MenuAction) => void;
  closeAll: () => void;
  navigate: (direction: 1 | -1) => void;
}

export interface MenuBarMenu {
  id: string;
  label: string;
  items: readonly MenuEntry[];
}

export const menuContextKey: InjectionKey<MenuContext> = Symbol('menu-context');
