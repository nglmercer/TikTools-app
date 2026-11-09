<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LiveFeedEvent, LiveFeedModel } from '../model/live-feed.ts';

const props = defineProps<{ model: LiveFeedModel; canLookupPoints?: (event: LiveFeedEvent) => boolean; showAmbiguousViewer?: boolean }>();
const emit = defineEmits<{ lookupPoints: [event: LiveFeedEvent] }>();
const { locale } = useI18n();
const text = computed(() => locale.value === 'es' ? {
  title: 'Actividad del LIVE', empty: 'Todavía no hay actividad disponible.', gap: 'Se perdió parte de la actividad; la lista puede estar incompleta.',
  unavailable: 'No se pudo consultar la actividad del LIVE.', polling: 'Se consulta cada dos segundos y puede incluir actividad anterior.',
  kind: 'Tipo de actividad', all: 'Toda la actividad', search: 'Buscar en actividad visible', noMatches: 'Ninguna actividad visible coincide con los filtros.',
  creator: 'Creador autorizado',
  points: (id: string) => `Consultar puntos de @${id}`,
  ambiguousViewer: 'No se puede confirmar el ID del autor; búscalo manualmente.',
  kinds: { chat: 'Chat', gift: 'Regalo', like: 'Me gusta', member: 'Ingreso', social: 'Social' } as Record<string, string>,
} : {
  title: 'LIVE activity', empty: 'No activity is available yet.', gap: 'Some activity was missed; this list may be incomplete.',
  unavailable: 'LIVE activity is unavailable.', polling: 'Checked every two seconds; earlier activity may be included.',
  kind: 'Activity type', all: 'All activity', search: 'Search visible activity', noMatches: 'No visible activity matches these filters.',
  creator: 'Allowed creator',
  points: (id: string) => `Look up points for @${id}`,
  ambiguousViewer: 'The author ID cannot be confirmed; look it up manually.',
  kinds: { chat: 'Chat', gift: 'Gift', like: 'Like', member: 'Join', social: 'Social' } as Record<string, string>,
});
const selectedKind = ref('all');
const query = ref('');
const visibleEvents = computed(() => {
  const term = query.value.trim().toLocaleLowerCase();
  return props.model.events.value.filter((event) =>
    (selectedKind.value === 'all' || event.kind === selectedKind.value) &&
    (!term || [event.author, event.nickname ?? '', event.text].some((value) => value.toLocaleLowerCase().includes(term))));
});
watch(() => props.model.scopeRevision.value, () => { selectedKind.value = 'all'; query.value = ''; });
</script>

<template>
  <section v-if="model.allowed.value" class="live-feed" :aria-label="text.title">
    <h2>{{ text.title }}</h2>
    <p v-if="model.error.value" role="alert">{{ text.unavailable }}</p>
    <template v-else>
      <p v-if="model.gap.value" role="status">{{ text.gap }}</p>
      <div class="filters">
        <label v-if="model.creatorOptions.value.length">{{ text.creator }}
          <select :value="model.selectedCreator.value ?? ''" @change="model.selectCreator(($event.target as HTMLSelectElement).value)">
            <option v-for="creator in model.creatorOptions.value" :key="creator" :value="creator">{{ creator }}</option>
          </select>
        </label>
        <label>{{ text.kind }}
          <select v-model="selectedKind">
            <option value="all">{{ text.all }}</option>
            <option v-for="(label, kind) in text.kinds" :key="kind" :value="kind">{{ label }}</option>
          </select>
        </label>
        <label>{{ text.search }}
          <input v-model="query" type="search" maxlength="128" autocomplete="off">
        </label>
      </div>
      <p v-if="model.events.value.length === 0">{{ text.empty }}</p>
      <p v-else-if="visibleEvents.length === 0">{{ text.noMatches }}</p>
      <ol v-else role="log" :aria-label="text.title">
        <li v-for="event in visibleEvents" :key="event.cursor">
          <strong>{{ event.nickname || event.author }}</strong>
          <span class="kind">{{ text.kinds[event.kind] }}</span>
          <span>{{ event.text }}</span>
          <button v-if="event.authorLookupId && canLookupPoints?.(event)" type="button" @click="emit('lookupPoints', event)">{{ text.points(event.authorLookupId) }}</button>
          <span v-else-if="showAmbiguousViewer && event.authorLookupId === null" class="ambiguous">{{ text.ambiguousViewer }}</span>
        </li>
      </ol>
    </template>
    <p>{{ text.polling }}</p>
  </section>
</template>

<style scoped>
.live-feed { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; }
.filters { display:flex; flex-wrap:wrap; gap:12px; margin:12px 0; }
.filters label { display:flex; flex-direction:column; gap:4px; }
ol { max-height:22rem; overflow-y:auto; padding-left:1.5rem; }
li { padding:0.25rem 0; white-space:pre-wrap; } .kind { margin:0 0.5rem; opacity:0.7; }
li button { margin-left:0.5rem; }
li .ambiguous { display:block; opacity:0.8; }
</style>
