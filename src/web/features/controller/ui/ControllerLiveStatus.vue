<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LiveStatusModel } from '../model/live.ts';

defineProps<{ model: LiveStatusModel }>();
const { locale } = useI18n();
const text = computed(() => locale.value === 'es' ? {
  title: 'LIVE del anfitrión', connected: 'Conectado al LIVE', disconnected: 'Sin conexión al LIVE',
  creator: 'Creador', loading: 'Consultando el estado del LIVE…', unavailable: 'No se pudo consultar el estado del LIVE.',
  polling: 'El estado se consulta periódicamente mientras el controlador está conectado.',
} : {
  title: 'Host LIVE', connected: 'Connected to LIVE', disconnected: 'Not connected to LIVE',
  creator: 'Creator', loading: 'Checking LIVE status…', unavailable: 'LIVE status is unavailable.',
  polling: 'Status is checked periodically while the controller is connected.',
});
</script>

<template>
  <section
    v-if="model.allowed.value"
    class="live-status"
    :aria-label="text.title"
  >
    <h2>{{ text.title }}</h2>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ text.unavailable }}
    </p>
    <p
      v-else-if="!model.data.value"
      role="status"
    >
      {{ text.loading }}
    </p>
    <template v-else>
      <p role="status">
        {{ model.data.value.connected ? text.connected : text.disconnected }}
      </p>
      <p v-if="model.data.value.uniqueId">
        {{ text.creator }}: {{ model.data.value.uniqueId }}
      </p>
    </template>
    <p>{{ text.polling }}</p>
  </section>
</template>

<style scoped>
.live-status { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; }
</style>
