<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';
import { onLongPress } from '@/shared/lib/touch-gestures.ts';
import { pointAnchor, type AnchorTarget } from '@/shared/lib/anchored-position.ts';
import BaseMenu from './BaseMenu.vue';
import type { MenuAction, MenuEntry } from './menu-types.ts';

const props = defineProps<{
  items: readonly MenuEntry[];
  disabled?: boolean;
}>();

const emit = defineEmits<{
  open: [event: MouseEvent | KeyboardEvent];
  select: [action: MenuAction];
}>();

const open = ref(false);
const anchor = shallowRef<AnchorTarget | null>(null);
const focusFirst = ref(false);
const generation = ref(0);

function show(target: AnchorTarget, event: MouseEvent | KeyboardEvent, keyboard: boolean): void {
  event.preventDefault();
  emit('open', event);
  anchor.value = target;
  focusFirst.value = keyboard;
  generation.value += 1;
  open.value = true;
}

/** Set by a touch long-press so Android's follow-up `contextmenu` does not reopen. */
let longPressAt = 0;

function onContextMenu(event: MouseEvent): void {
  if (props.disabled) return;
  if (Date.now() - longPressAt < 1000) {
    event.preventDefault();
    return;
  }
  show(pointAnchor(event.clientX, event.clientY), event, false);
}

// iOS never fires `contextmenu` for touch: a long-press opens the menu instead.
const root = ref<HTMLElement | null>(null);
let disposeLongPress: (() => void) | null = null;
onMounted(() => {
  if (!root.value) return;
  disposeLongPress = onLongPress(root.value, {
    onLongPress: ({ x, y }, target) => {
      if (props.disabled) return;
      // Let the row under the finger update the selection first, as a right-click does.
      target?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: x, clientY: y }));
      longPressAt = Date.now();
    },
  });
});
onBeforeUnmount(() => disposeLongPress?.());

function onKeydown(event: KeyboardEvent): void {
  if (props.disabled) return;
  if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return;
  const target = event.target as HTMLElement;
  const rect = target.getBoundingClientRect();
  show(pointAnchor(rect.left + 8, rect.bottom), event, true);
}
</script>

<template>
  <div
    ref="root"
    class="base-context-menu"
    @contextmenu="onContextMenu"
    @keydown="onKeydown"
  >
    <slot />
    <BaseMenu
      :key="generation"
      v-model:open="open"
      :items="items"
      :anchor="anchor"
      :focus-first="focusFirst"
      @select="emit('select', $event)"
    />
  </div>
</template>

<style scoped>
.base-context-menu {
  display: contents;
}
</style>
