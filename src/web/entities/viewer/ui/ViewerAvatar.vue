<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { avatarColor, avatarInitials } from '../lib/avatar-color.ts';

const props = withDefaults(
  defineProps<{
    handle: string;
    src?: string | null;
    size?: number;
  }>(),
  { src: undefined, size: 24 },
);

const failed = ref(false);
watch(
  () => props.src,
  () => {
    failed.value = false;
  },
);

const showImage = computed(() => Boolean(props.src) && !failed.value);
</script>

<template>
  <span
    class="viewer-avatar"
    :style="{ width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.4)}px` }"
    aria-hidden="true"
  >
    <img
      v-if="showImage"
      class="viewer-avatar__image"
      :src="src!"
      alt=""
      decoding="async"
      referrerpolicy="no-referrer"
      @error="failed = true"
    >
    <span
      v-else
      class="viewer-avatar__initials"
      :style="{ background: avatarColor(handle) }"
    >{{ avatarInitials(handle) }}</span>
  </span>
</template>

<style scoped>
.viewer-avatar {
  display: inline-flex;
  flex: none;
  overflow: hidden;
  border-radius: 50%;
  background: var(--color-bg-control);
}

.viewer-avatar__image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.viewer-avatar__initials {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  color: var(--color-bg-app);
  font-family: var(--font-family);
  font-weight: var(--font-weight-semibold);
}
</style>
