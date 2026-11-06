<script setup lang="ts">
import { onScopeDispose, provide, useId, watch } from 'vue';
import type { AnchorTarget, Placement } from '@/shared/lib/anchored-position.ts';
import MenuList from './MenuList.vue';
import { menuContextKey, type MenuAction, type MenuEntry } from './menu-types.ts';

const props = withDefaults(
  defineProps<{
    items: readonly MenuEntry[];
    anchor: AnchorTarget | null;
    placement?: Placement;
    focusFirst?: boolean;
  }>(),
  { placement: 'bottom-start' },
);

const emit = defineEmits<{ select: [action: MenuAction]; navigate: [direction: 1 | -1] }>();
const open = defineModel<boolean>('open', { default: false });

const rootId = useId();
let returnFocus: HTMLElement | null = null;
let restoreFocus = true;

function closeAll(restore = true): void {
  if (!open.value) return;
  restoreFocus = restore;
  open.value = false;
}

const dismiss = (): void => closeAll(false);

function select(action: MenuAction): void {
  closeAll();
  emit('select', action);
}

provide(menuContextKey, {
  rootId,
  select,
  closeAll: () => closeAll(),
  navigate: (direction) => emit('navigate', direction),
});

const onPointerDown = (event: PointerEvent): void => {
  const target = event.target as Element | null;
  if (target?.closest(`[data-menu-root="${rootId}"]`)) return;
  if (props.anchor instanceof Element && target && props.anchor.contains(target)) return;
  dismiss();
};

function listen(active: boolean): void {
  if (active) {
    document.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('blur', dismiss);
    window.addEventListener('resize', dismiss);
  } else {
    document.removeEventListener('pointerdown', onPointerDown, true);
    window.removeEventListener('blur', dismiss);
    window.removeEventListener('resize', dismiss);
  }
}

watch(
  open,
  (isOpen, wasOpen) => {
    listen(false);
    if (isOpen) {
      returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      restoreFocus = true;
      listen(true);
    } else if (wasOpen) {
      if (restoreFocus) returnFocus?.focus();
      returnFocus = null;
    }
  },
  { immediate: true },
);

onScopeDispose(() => listen(false));
</script>

<template>
  <!-- A menu with no entries (e.g. a context target without commands) shows nothing. -->
  <MenuList
    v-if="open && items.length"
    :items="items"
    :anchor="anchor"
    :placement="placement"
    :focus-first="focusFirst"
  />
</template>
