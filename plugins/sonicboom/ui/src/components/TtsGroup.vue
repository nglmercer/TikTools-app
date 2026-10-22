<script setup lang="ts">
import TtsIcon from './TtsIcon.vue';
defineProps<{ title: string; icon: string; expanded?: boolean }>();
</script>
<template>
  <details class="tts-disclosure" :open="expanded">
    <summary><TtsIcon :name="icon"/><span>{{ title }}</span><TtsIcon name="chevron" class="tts-disclosure__chevron"/></summary>
    <div class="tts-group__body"><slot/></div>
  </details>
</template>
