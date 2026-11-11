<script setup lang="ts">
import { markRaw, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';

/**
 * One toolbar button for every host history and review (adoptions, definitions, library, approvals),
 * instead of one text button each. A dot marks that something still needs attention.
 */
defineProps<{ items: readonly MenuEntry[]; attention?: boolean; disabled?: boolean }>();
const emit = defineEmits<{ select: [id: string] }>();

const { t } = useI18n();
const open = ref(false);
const anchor = shallowRef<HTMLElement | null>(null);

function toggle(event: MouseEvent): void {
  anchor.value = markRaw(event.currentTarget as HTMLElement);
  open.value = !open.value;
}

function onSelect(action: MenuAction): void {
  emit('select', action.id);
}
</script>

<template>
  <span class="history-menu">
    <BaseIconButton
      :label="t('automations.history.menu')"
      variant="default"
      aria-haspopup="menu"
      :aria-expanded="open"
      :disabled="disabled"
      data-test="history-menu"
      @click="toggle"
    >
      <IconHistory />
    </BaseIconButton>
    <span
      v-if="attention"
      class="history-menu__dot"
      aria-hidden="true"
    />
    <BaseMenu
      v-model:open="open"
      :items="items"
      :anchor="anchor"
      placement="bottom-start"
      @select="onSelect"
    />
  </span>
</template>

<style scoped>
.history-menu {
  position: relative;
  display: inline-flex;
}

.history-menu__dot {
  position: absolute;
  top: 3px;
  right: 3px;
  width: 8px;
  height: 8px;
  border: var(--border-width) solid var(--color-bg-control);
  border-radius: 50%;
  background: var(--color-warning);
  pointer-events: none;
}
</style>
