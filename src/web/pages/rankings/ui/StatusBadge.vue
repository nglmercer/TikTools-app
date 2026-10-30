<script setup lang="ts">
defineProps<{ label: string; tone?: 'ok' | 'warn' | 'danger' | 'muted' }>();
</script>

<template>
  <span
    class="status-badge"
    :class="`status-badge--${tone ?? 'muted'}`"
  >{{ label }}</span>
</template>

<style scoped>
.status-badge { padding: 2px 10px; border-radius: 999px; font-size: 12px; font-weight: 600; white-space: nowrap; }
.status-badge--ok { background: color-mix(in srgb, #22c55e 20%, transparent); color: #4ade80; }
.status-badge--warn { background: color-mix(in srgb, #f59e0b 20%, transparent); color: #fbbf24; }
.status-badge--danger { background: color-mix(in srgb, #ef4444 18%, transparent); color: #f87171; }
.status-badge--muted { background: var(--color-bg-control); color: var(--color-text-muted); }
</style>
