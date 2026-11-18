<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import type { AnchorTarget, Placement } from '@/shared/lib/anchored-position.ts';
import { firstEnabledIndex, lastEnabledIndex, stepEnabledIndex, typeaheadIndex } from '@/shared/lib/list-navigation.ts';
import { useAnchoredPosition } from '@/shared/lib/use-anchored-position.ts';
import { isMenuAction, menuContextKey, type MenuEntry } from './menu-types.ts';

const props = withDefaults(
  defineProps<{
    items: readonly MenuEntry[];
    anchor: AnchorTarget | null;
    placement?: Placement;
    focusFirst?: boolean;
    nested?: boolean;
  }>(),
  { placement: 'bottom-start' },
);

const emit = defineEmits<{ close: [] }>();

const context = inject(menuContextKey)!;
const list = useTemplateRef('list');
const itemElements = ref<HTMLElement[]>([]);
const activeIndex = ref(-1);
const submenuIndex = ref<number | null>(null);
const anchor = computed(() => props.anchor);
const { style } = useAnchoredPosition(anchor, list, ref(true), { placement: props.placement, offset: props.nested ? -2 : 1 });

const navigable = computed(() =>
  props.items.map((entry) => (isMenuAction(entry) ? entry : { disabled: true })),
);
const submenuItems = computed(() => {
  if (submenuIndex.value === null) return null;
  const entry = props.items[submenuIndex.value];
  return entry && isMenuAction(entry) ? (entry.children ?? null) : null;
});

let hoverTimer: ReturnType<typeof setTimeout> | undefined;
let submenuFocusFirst = false;

function focusItem(index: number): void {
  activeIndex.value = index;
  itemElements.value[index]?.focus({ preventScroll: true });
}

function openSubmenu(index: number, focusFirst: boolean): void {
  const entry = props.items[index];
  if (!entry || !isMenuAction(entry) || entry.disabled || !entry.children?.length) return;
  submenuFocusFirst = focusFirst;
  submenuIndex.value = index;
}

function closeSubmenu(): void {
  if (submenuIndex.value === null) return;
  const index = submenuIndex.value;
  submenuIndex.value = null;
  focusItem(index);
}

function activate(index: number): void {
  const entry = props.items[index];
  if (!entry || !isMenuAction(entry) || entry.disabled) return;
  if (entry.children?.length) openSubmenu(index, true);
  else context.select(entry);
}

function onPointerEnter(index: number): void {
  const entry = props.items[index];
  if (!entry || !isMenuAction(entry)) return;
  focusItem(index);
  clearTimeout(hoverTimer);
  hoverTimer = setTimeout(() => {
    if (entry.children?.length && !entry.disabled) openSubmenu(index, false);
    else submenuIndex.value = null;
  }, 150);
}

let typeahead = '';
let typeaheadTimer: ReturnType<typeof setTimeout> | undefined;

function onTypeahead(key: string): void {
  clearTimeout(typeaheadTimer);
  typeahead += key;
  typeaheadTimer = setTimeout(() => {
    typeahead = '';
  }, 500);
  const index = typeaheadIndex(navigable.value, typeahead, activeIndex.value);
  if (index >= 0) focusItem(index);
}

function onKeydown(event: KeyboardEvent): void {
  const actions: Record<string, () => void> = {
    ArrowDown: () => focusItem(stepEnabledIndex(navigable.value, activeIndex.value, 1)),
    ArrowUp: () =>
      focusItem(
        activeIndex.value < 0
          ? lastEnabledIndex(navigable.value)
          : stepEnabledIndex(navigable.value, activeIndex.value, -1),
      ),
    Home: () => focusItem(firstEnabledIndex(navigable.value)),
    End: () => focusItem(lastEnabledIndex(navigable.value)),
    ArrowRight: () => {
      const entry = props.items[activeIndex.value];
      if (entry && isMenuAction(entry) && entry.children?.length && !entry.disabled) openSubmenu(activeIndex.value, true);
      else context.navigate(1);
    },
    ArrowLeft: () => (props.nested ? emit('close') : context.navigate(-1)),
    Escape: () => (props.nested ? emit('close') : context.closeAll()),
    Enter: () => activate(activeIndex.value),
    ' ': () => activate(activeIndex.value),
    Tab: context.closeAll,
  };
  const action = actions[event.key];
  if (action) {
    event.preventDefault();
    event.stopPropagation();
    action();
  } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
    onTypeahead(event.key);
  }
}

onMounted(() => {
  if (props.focusFirst) focusItem(firstEnabledIndex(navigable.value));
  else list.value?.focus({ preventScroll: true });
});

onBeforeUnmount(() => {
  clearTimeout(hoverTimer);
  clearTimeout(typeaheadTimer);
});
</script>

<template>
  <Teleport to="body">
    <ul
      ref="list"
      class="menu-list"
      role="menu"
      tabindex="-1"
      :data-menu-root="context.rootId"
      :style="style"
      @keydown="onKeydown"
      @contextmenu.prevent
    >
      <template
        v-for="(entry, index) in items"
        :key="entry.id"
      >
        <li
          v-if="!isMenuAction(entry)"
          class="menu-list__separator"
          role="separator"
        />
        <li
          v-else
          :ref="(element) => (itemElements[index] = element as HTMLElement)"
          class="menu-list__item"
          :class="{ 'menu-list__item--active': index === activeIndex || index === submenuIndex }"
          :role="entry.checked === undefined ? 'menuitem' : 'menuitemcheckbox'"
          tabindex="-1"
          :aria-checked="entry.checked"
          :aria-disabled="entry.disabled || undefined"
          :aria-haspopup="entry.children?.length ? 'menu' : undefined"
          :aria-expanded="entry.children?.length ? index === submenuIndex : undefined"
          @pointerenter="onPointerEnter(index)"
          @click="activate(index)"
        >
          <span class="menu-list__icon">
            <IconCheck v-if="entry.checked" />
            <component
              :is="entry.icon"
              v-else-if="entry.icon"
            />
          </span>
          <span class="menu-list__label">{{ entry.label }}</span>
          <span
            v-if="entry.shortcut"
            class="menu-list__shortcut"
          >{{ entry.shortcut }}</span>
          <IconChevronRight
            v-if="entry.children?.length"
            class="menu-list__submenu"
            :size="12"
          />
        </li>
      </template>
    </ul>
    <MenuList
      v-if="submenuItems && submenuIndex !== null"
      :key="submenuIndex"
      :items="submenuItems"
      :anchor="itemElements[submenuIndex] ?? null"
      placement="right-start"
      :focus-first="submenuFocusFirst"
      nested
      @close="closeSubmenu"
    />
  </Teleport>
</template>

<style scoped>
.menu-list {
  z-index: var(--z-popover);
  min-width: 180px;
  /* Keep the popup inside the viewport at any width; long labels wrap instead of clipping. */
  max-width: min(360px, calc(100vw - 2rem));
  max-height: min(70vh, 480px);
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  margin: 0;
  padding: var(--space-1) 0;
  border: var(--border-width) solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-bg-overlay);
  box-shadow: var(--shadow-popup);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  list-style: none;
  outline: none;
  user-select: none;
}

.menu-list__item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: var(--control-height-md);
  margin: 0 var(--space-1);
  padding: var(--space-2) var(--space-4) var(--space-2) var(--space-3);
  border: var(--border-width) solid transparent;
  border-radius: var(--radius-sm);
  cursor: default;
  outline: none;
}

.menu-list__item--active {
  border-color: var(--color-border-selected);
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

.menu-list__item[aria-disabled='true'] {
  border-color: transparent;
  background: transparent;
  color: var(--color-text-disabled);
}

.menu-list__icon {
  display: inline-flex;
  flex: none;
  width: var(--icon-size);
  justify-content: center;
}

.menu-list__label {
  flex: 1;
  min-width: 0;
  white-space: normal;
  overflow-wrap: anywhere;
}

.menu-list__shortcut {
  flex: none;
  padding-left: var(--space-7);
  color: var(--color-text-muted);
  white-space: nowrap;
}

.menu-list__item--active .menu-list__shortcut {
  color: inherit;
}

.menu-list__submenu {
  margin-right: calc(var(--space-2) * -1);
}

.menu-list__separator {
  height: var(--border-width);
  margin: var(--space-2) var(--space-3);
  background: var(--color-border);
}
</style>
