<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
const props = withDefaults(defineProps<{ title: string; initiallyOpen?: boolean; open?: boolean }>(), { open: undefined });
const emit = defineEmits<{ 'update:open': [value: boolean] }>();
const localOpen = ref(props.initiallyOpen ?? false);
const expanded = computed({ get: () => props.open ?? localOpen.value, set: (value: boolean) => { localOpen.value = value; emit('update:open', value); } });
const id = useId();
</script>

<template>
  <section class="settings-disclosure">
    <button
      :id="`${id}-toggle`"
      type="button"
      class="settings-disclosure__toggle"
      :aria-expanded="expanded"
      :aria-controls="`${id}-content`"
      @click="expanded = !expanded"
    >
      <IconChevronRight
        :class="{ 'settings-disclosure__arrow--open': expanded }"
        aria-hidden="true"
      />
      <span>{{ title }}</span>
    </button>
    <div
      v-show="expanded"
      :id="`${id}-content`"
      class="settings-disclosure__content"
      role="region"
      :aria-labelledby="`${id}-toggle`"
    >
      <slot />
    </div>
  </section>
</template>

<style scoped>
.settings-disclosure { min-width: 0; border: var(--border-width) solid var(--color-border-subtle); background: var(--color-bg-panel); }
.settings-disclosure__toggle { display: flex; align-items: center; gap: var(--space-3); width: 100%; min-height: 36px; padding: var(--space-3) var(--space-4); border: 0; color: var(--color-text); background: var(--color-bg-panel-header); font: inherit; font-weight: var(--font-weight-semibold); text-align: left; cursor: pointer; }
.settings-disclosure__toggle:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: -2px; }
.settings-disclosure__toggle:hover { background: var(--color-bg-hover); }
.settings-disclosure__toggle svg { flex: none; }
.settings-disclosure__arrow--open { transform: rotate(90deg); }
.settings-disclosure__content { min-width: 0; padding: var(--space-5); }
@media (any-pointer: coarse), (width < 700px) { .settings-disclosure__toggle { min-height: 44px; } }
</style>
