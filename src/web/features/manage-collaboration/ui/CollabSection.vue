<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ title: string; description?: string }>();
const id = useId();
</script>

<template>
  <section
    class="collab-section"
    role="group"
    :aria-labelledby="`${id}-title`"
  >
    <header class="collab-section__head">
      <div class="collab-section__heading">
        <h3
          :id="`${id}-title`"
          class="collab-section__title"
        >
          {{ title }}
        </h3>
        <p
          v-if="description"
          class="collab-section__description"
        >
          {{ description }}
        </p>
      </div>
      <div
        v-if="$slots.actions"
        class="collab-section__actions"
      >
        <slot name="actions" />
      </div>
    </header>
    <div class="collab-section__body">
      <slot />
    </div>
    <footer
      v-if="$slots.footer"
      class="collab-section__footer"
    >
      <slot name="footer" />
    </footer>
  </section>
</template>
