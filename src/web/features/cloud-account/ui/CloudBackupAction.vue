<script setup lang="ts">
import type { Component } from 'vue';

/** One file action: what it does, in a sentence, and the button that does it. */
defineProps<{ icon: Component; title: string; description: string }>();
</script>

<template>
  <div class="backup-action">
    <span
      class="backup-action__icon"
      aria-hidden="true"
    >
      <component
        :is="icon"
        :size="18"
      />
    </span>
    <div class="backup-action__text">
      <strong class="backup-action__title">{{ title }}</strong>
      <p class="backup-action__description">
        {{ description }}
      </p>
    </div>
    <div class="backup-action__control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.backup-action {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-raised);
}
.backup-action__icon {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: var(--color-bg-control);
  color: var(--color-accent);
}
.backup-action__text { flex: 1 1 220px; min-width: 0; }
.backup-action__title { font-weight: var(--font-weight-semibold); }
.backup-action__description { margin: var(--space-1) 0 0; color: var(--color-text-muted); font-size: var(--font-size-sm); line-height: var(--line-height-normal); }
.backup-action__control { flex: none; }
</style>
