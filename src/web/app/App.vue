<template>
  <RouterView />
  <SessionRecoveryDialog />
</template>

<script setup lang="ts">
import SessionRecoveryDialog from '@/shared/ui/dialog/SessionRecoveryDialog.vue';
</script>
