<script setup lang="ts">
import { computed, shallowRef } from 'vue';
import { resolveChatSegments } from '../../../../shared/emotes/resolve.ts';
import type { LiveEmote } from '../../../../shared/emotes/types.ts';
import { hasEmoteImageFailed, markEmoteImageFailed } from '../lib/emote-failures.ts';

const props = withDefaults(
  defineProps<{
    text: string;
    emotes?: LiveEmote[];
    images?: boolean;
  }>(),
  { emotes: undefined, images: true },
);

const localFailures = shallowRef<ReadonlySet<string>>(new Set());
function imageFailed(url: string): void {
  markEmoteImageFailed(url);
  localFailures.value = new Set([...localFailures.value, url].slice(-64));
}

const segments = computed(() => resolveChatSegments(props.text, props.emotes));

function showImage(url: string | undefined): url is string {
  return Boolean(props.images && url && !localFailures.value.has(url) && !hasEmoteImageFailed(url));
}
</script>

<template>
  <template
    v-for="(segment, index) in segments"
    :key="index"
  >
    <template v-if="segment.type === 'text'">
      {{ segment.value }}
    </template>
    <img
      v-else-if="showImage(segment.imageUrl)"
      class="chat-emote"
      :src="segment.imageUrl"
      :alt="segment.fallbackText || 'TikTok emote'"
      :title="segment.fallbackText || 'TikTok emote'"
      decoding="async"
      referrerpolicy="no-referrer"
      @error="imageFailed(segment.imageUrl!)"
    >
    <template v-else-if="segment.fallbackText">
      {{ segment.fallbackText }}
    </template>
    <span
      v-else
      class="chat-emote chat-emote--fallback"
      role="img"
      :aria-label="segment.shortcode || 'TikTok emote'"
    >◻</span>
  </template>
</template>

<style scoped>
.chat-emote {
  display: inline-block;
  width: 1.35em;
  height: 1.35em;
  margin: 0 0.08em;
  vertical-align: -0.3em;
  object-fit: contain;
}
</style>
