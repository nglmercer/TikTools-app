<script setup lang="ts">
defineProps<{ name: string }>();
const paths: Record<string, string[]> = {
  voice: ['M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3Z', 'M5 10v2a7 7 0 0 0 14 0v-2M12 19v3M8 22h8'],
  users: ['M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3a4 4 0 0 1 0 8M22 21v-2a4 4 0 0 0-3-3.87', 'M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z'],
  audio: ['M11 5 6 9H3v6h3l5 4V5ZM16 8a6 6 0 0 1 0 8M19 5a10 10 0 0 1 0 14'],
  message: ['M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.4 8.4 0 0 1 3.8-.9h.5a8.5 8.5 0 0 1 8 8v.5Z'],
  sparkle: ['m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3Z'],
  coins: ['M20 7c0 2-3.6 4-8 4S4 9 4 7s3.6-4 8-4 8 2 8 4ZM4 7v5c0 2 3.6 4 8 4s8-2 8-4V7M4 12v5c0 2 3.6 4 8 4s8-2 8-4v-5'],
  sliders: ['M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6'],
  play: ['m8 5 12 7-12 7V5Z'], pause: ['M8 5v14M16 5v14'], skip: ['m5 5 10 7-10 7V5ZM19 5v14'],
  stop: ['M6 6h12v12H6Z'], refresh: ['M20 7v5h-5M4 17v-5h5M6 7a7 7 0 0 1 11.6-2L20 8M4 16l2.4 3A7 7 0 0 0 18 17'],
  check: ['m5 12 4 4L19 6'], chevron: ['m6 9 6 6 6-6'], info: ['M12 11v6M12 7h.01', 'M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0Z'],
};
</script>
<template><svg class="tts-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path v-for="(path, index) in paths[name] || paths.sliders" :key="index" :d="path"/></svg></template>
