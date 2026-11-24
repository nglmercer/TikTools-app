<script setup lang="ts">
import BaseIcon from '../BaseIcon.vue';
</script>

<template>
  <BaseIcon>
    <g
      fill="currentColor"
      stroke="none"
    >
      <circle
        v-for="y in [3.5, 8, 12.5]"
        :key="`left-${y}`"
        cx="5.5"
        :cy="y"
        r="1.25"
      />
      <circle
        v-for="y in [3.5, 8, 12.5]"
        :key="`right-${y}`"
        cx="10.5"
        :cy="y"
        r="1.25"
      />
    </g>
  </BaseIcon>
</template>
