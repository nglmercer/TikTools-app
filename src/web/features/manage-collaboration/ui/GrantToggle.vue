<script setup lang="ts">
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
// One permission: the switch carries only its label (so it stays a clean accessible name);
// its explanation sits directly under it instead of floating between unrelated switches.
defineProps<{ disabled?: boolean; hint?: string; nested?: boolean }>();
const model = defineModel<boolean>({ default: false });
</script>

<template>
  <div
    class="grant-toggle"
    :class="{ 'grant-toggle--nested': nested, 'grant-toggle--on': model, 'grant-toggle--disabled': disabled }"
  >
    <BaseSwitch
      v-model="model"
      :disabled="disabled"
    >
      <slot />
    </BaseSwitch>
    <p
      v-if="hint"
      class="grant-toggle__hint"
    >
      {{ hint }}
    </p>
  </div>
</template>
