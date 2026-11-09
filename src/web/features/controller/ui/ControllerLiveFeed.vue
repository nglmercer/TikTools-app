<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LiveFeedModel } from '../model/live-feed.ts';

defineProps<{ model: LiveFeedModel }>();
const { locale } = useI18n();
const text = computed(() => locale.value === 'es' ? {
  title: 'Actividad del LIVE', empty: 'Todavía no hay actividad disponible.', gap: 'Se perdió parte de la actividad; la lista puede estar incompleta.',
  unavailable: 'No se pudo consultar la actividad del LIVE.', polling: 'Se consulta cada dos segundos y puede incluir actividad anterior.',
  kinds: { chat: 'Chat', gift: 'Regalo', like: 'Me gusta', member: 'Ingreso', social: 'Social' } as Record<string, string>,
} : {
  title: 'LIVE activity', empty: 'No activity is available yet.', gap: 'Some activity was missed; this list may be incomplete.',
  unavailable: 'LIVE activity is unavailable.', polling: 'Checked every two seconds; earlier activity may be included.',
  kinds: { chat: 'Chat', gift: 'Gift', like: 'Like', member: 'Join', social: 'Social' } as Record<string, string>,
});
</script>

<template>
  <section v-if="model.allowed.value" class="live-feed" :aria-label="text.title">
    <h2>{{ text.title }}</h2>
    <p v-if="model.error.value" role="alert">{{ text.unavailable }}</p>
    <template v-else>
      <p v-if="model.gap.value" role="status">{{ text.gap }}</p>
      <p v-if="model.events.value.length === 0">{{ text.empty }}</p>
      <ol v-else role="log" :aria-label="text.title">
        <li v-for="event in model.events.value" :key="event.cursor">
          <strong>{{ event.nickname || event.author }}</strong>
          <span class="kind">{{ text.kinds[event.kind] }}</span>
          <span>{{ event.text }}</span>
        </li>
      </ol>
    </template>
    <p>{{ text.polling }}</p>
  </section>
</template>

<style scoped>
.live-feed { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; }
ol { max-height:22rem; overflow-y:auto; padding-left:1.5rem; }
li { padding:0.25rem 0; white-space:pre-wrap; } .kind { margin:0 0.5rem; opacity:0.7; }
</style>
