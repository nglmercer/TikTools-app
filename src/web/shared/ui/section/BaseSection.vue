<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <section class="base-section">
    <header class="base-section__header">
      <h3 class="base-section__title">
        {{ title }}
      </h3>
      <span
        v-if="$slots.actions"
        class="base-section__actions"
      >
        <slot name="actions" />
      </span>
    </header>
    <div class="base-section__body">
      <p
        v-if="description"
        class="base-section__description"
      >
        {{ description }}
      </p>
      <slot />
    </div>
  </section>
</template>

<style scoped>
.base-section {
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.base-section__header {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
  min-height: var(--control-height-md);
  padding: 0 var(--space-4);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
}

.base-section__title {
  margin: 0;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  overflow-wrap: anywhere;
}

.base-section__actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
}

.base-section__body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-3);
  min-width: 0;
  padding: var(--space-4);
}

.base-section__description {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}
</style>
