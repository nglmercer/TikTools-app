<script setup lang="ts">
// A single row in the devices list: identity on the left, state and actions on the right,
// details underneath. Keeps every record type visually consistent.
defineProps<{ initial?: string }>();
</script>

<template>
  <article class="collab-record">
    <span
      v-if="initial"
      class="collab-record__avatar"
      aria-hidden="true"
    >{{ initial }}</span>
    <div class="collab-record__main">
      <header class="collab-record__head">
        <h4 class="collab-record__title">
          <slot name="title" />
        </h4>
        <span
          v-if="$slots.status"
          class="collab-record__status"
        ><slot name="status" /></span>
      </header>
      <div
        v-if="$slots.meta"
        class="collab-record__meta"
      >
        <slot name="meta" />
      </div>
      <div
        v-if="$slots.default"
        class="collab-record__body"
      >
        <slot />
      </div>
    </div>
    <div
      v-if="$slots.actions"
      class="collab-record__actions"
    >
      <slot name="actions" />
    </div>
  </article>
</template>
