<script setup lang="ts">
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import type { StatusTone } from '@/shared/ui/status/status-types.ts';
defineProps<{ label: string; tone?: 'ok' | 'warn' | 'danger' | 'muted' }>();
const tones: Record<'ok' | 'warn' | 'danger' | 'muted', StatusTone> = { ok: 'success', warn: 'warning', danger: 'error', muted: 'neutral' };
</script>
<template>
  <BaseStatus
    :label="label"
    :tone="tones[tone ?? 'muted']"
  />
</template>
