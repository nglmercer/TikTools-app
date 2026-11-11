<script setup lang="ts">
import { useId } from 'vue';

defineProps<{
  title: string;
  subtitle?: string;
  flush?: boolean;
  headerLayout?: 'auto' | 'inline' | 'stacked';
}>();

const titleId = useId();
</script>

<template>
  <section
    class="base-panel"
    :class="`base-panel--${headerLayout ?? 'auto'}`"
    :aria-labelledby="titleId"
  >
    <header class="base-panel__header">
      <slot name="leading" />
      <h2
        :id="titleId"
        class="base-panel__title"
      >
        {{ title }}
      </h2>
      <span
        v-if="$slots.status"
        class="base-panel__status"
      >
        <slot name="status" />
      </span>
      <span
        v-if="subtitle"
        class="base-panel__subtitle"
      >{{ subtitle }}</span>
      <span
        v-if="$slots.actions"
        class="base-panel__actions"
      >
        <slot name="actions" />
      </span>
    </header>
    <div
      class="base-panel__body"
      :class="{ 'base-panel__body--flush': flush }"
    >
      <slot />
    </div>
    <footer
      v-if="$slots.footer"
      class="base-panel__footer"
    >
      <slot name="footer" />
    </footer>
  </section>
</template>

<style scoped>
.base-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.base-panel__header {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
  min-height: var(--control-height-lg);
  padding: var(--space-1) var(--space-2) var(--space-1) var(--space-4);
  background: var(--color-bg-panel-header);
}

.base-panel__title {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
  overflow-wrap: anywhere;
}

.base-panel__subtitle {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-panel__status {
  display: flex;
  align-items: center;
  min-width: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-regular);
}

.base-panel__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
}

.base-panel__body {
  flex: 1;
  min-height: 0;
  padding: var(--space-4);
  overflow: auto;
}

.base-panel__body--flush {
  padding: 0;
}

.base-panel__footer {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.base-panel--auto .base-panel__header { flex-wrap: wrap; }
.base-panel--stacked .base-panel__header { flex-wrap: wrap; }
.base-panel--stacked .base-panel__actions { flex-basis: 100%; margin-left: 0; }
.base-panel__actions { min-width: 0; flex-wrap: wrap; }
@media (width < 600px) {
  .base-panel--auto .base-panel__actions { flex-basis: 100%; margin-left: 0; }
  .base-panel--auto .base-panel__subtitle { white-space: normal; }
}
</style>
