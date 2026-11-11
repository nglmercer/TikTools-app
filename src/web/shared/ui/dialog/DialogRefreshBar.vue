<script setup lang="ts">
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';

/**
 * Shared strip for host-history dialogs: what the history belongs to on the left (the host identity),
 * and a refresh icon on the right instead of a full-width "Refresh" button.
 */
defineProps<{ label: string; busy?: boolean }>();
const emit = defineEmits<{ refresh: [] }>();
</script>

<template>
  <div class="dialog-refresh-bar">
    <span class="dialog-refresh-bar__identity"><slot /></span>
    <BaseIconButton
      :label="label"
      variant="default"
      size="sm"
      :disabled="busy"
      @click="emit('refresh')"
    >
      <IconRefresh />
    </BaseIconButton>
  </div>
</template>

<style scoped>
.dialog-refresh-bar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 0;
}

.dialog-refresh-bar__identity {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}
</style>
