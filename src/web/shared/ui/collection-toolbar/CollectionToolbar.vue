<script setup lang="ts">
defineProps<{ label: string; count?: string }>();
</script>

<template>
  <div
    class="collection-toolbar"
    role="group"
    :aria-label="label"
  >
    <div
      v-if="$slots.search"
      class="collection-toolbar__search"
    >
      <slot name="search" />
    </div>
    <div
      v-if="$slots.default"
      class="collection-toolbar__filters"
    >
      <slot />
    </div>
    <span
      v-if="count"
      class="collection-toolbar__count"
    >{{ count }}</span>
    <div
      v-if="$slots.actions"
      class="collection-toolbar__actions"
    >
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.collection-toolbar { display:flex; align-items:center; flex-wrap:wrap; flex:none; gap:var(--space-3); min-width:0; }
.collection-toolbar__search { flex:1 1 180px; min-width:0; }
.collection-toolbar__search > :deep(*) { width:100%; }
.collection-toolbar__filters,.collection-toolbar__actions { display:flex; align-items:center; flex-wrap:wrap; gap:var(--space-3); min-width:0; max-width:100%; }
.collection-toolbar__filters { flex:1 1 auto; }
.collection-toolbar__actions { margin-left:auto; }
.collection-toolbar__count { color:var(--color-text-muted); font-size:var(--font-size-sm); }
@media (width < 600px) { .collection-toolbar__search { flex-basis:100%; } .collection-toolbar__actions { margin-left:0; } }
</style>
