<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CompetitionModel } from '../model/competitions.ts';

defineProps<{ model: CompetitionModel }>();
const { locale } = useI18n();
const text = computed(() => locale.value === 'es' ? {
  title: 'Competiciones', description: 'Estado de las competiciones autorizadas.',
  loading: 'Consultando competiciones…', error: 'No se pudo consultar el estado de las competiciones.',
  empty: 'No hay competiciones disponibles en esta página.', previous: 'Anterior', next: 'Siguiente', page: 'Página',
  start: 'Iniciar', resume: 'Reanudar', pause: 'Pausar', cancel: 'Cancelar competición',
  states: { draft:'Borrador', active:'Activa', paused:'Pausada', finalizing:'Finalizando', finished:'Finalizada', cancelled:'Cancelada', failed:'Fallida' } as Record<string, string>,
  kinds: { poll:'Encuesta', weighted_poll:'Encuesta ponderada', leaderboard_challenge:'Desafío de ranking', team_battle:'Batalla de equipos', random_draw:'Sorteo', weighted_draw:'Sorteo ponderado', roulette:'Ruleta' } as Record<string, string>,
} : {
  title: 'Competitions', description: 'Status of permitted competitions.',
  loading: 'Loading competitions…', error: 'Competition status is unavailable.',
  empty: 'No competitions are available on this page.', previous: 'Previous', next: 'Next', page: 'Page',
  start: 'Start', resume: 'Resume', pause: 'Pause', cancel: 'Cancel competition',
  states: { draft:'Draft', active:'Active', paused:'Paused', finalizing:'Finalizing', finished:'Finished', cancelled:'Cancelled', failed:'Failed' } as Record<string, string>,
  kinds: { poll:'Poll', weighted_poll:'Weighted poll', leaderboard_challenge:'Leaderboard challenge', team_battle:'Team battle', random_draw:'Random draw', weighted_draw:'Weighted draw', roulette:'Roulette' } as Record<string, string>,
});
</script>

<template>
  <section
    v-if="model.allowed.value"
    class="controller-competitions"
    :aria-label="text.title"
  >
    <h2>{{ text.title }}</h2>
    <p>{{ text.description }}</p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ text.error }}
    </p>
    <p
      v-else-if="model.loading.value && !model.rows.value.length"
      role="status"
    >
      {{ text.loading }}
    </p>
    <p v-else-if="!model.rows.value.length">
      {{ text.empty }}
    </p>
    <ul v-else>
      <li
        v-for="row in model.rows.value"
        :key="row.id"
      >
        <strong>{{ row.name }}</strong>
        <span>{{ text.kinds[row.kind] }}</span>
        <span>{{ text.states[row.lifecycle] }}</span>
        <button
          v-if="model.canStop(row, 'start-competition')"
          type="button"
          @click="model.stopCompetition(row, 'start-competition')"
        >
          {{ text.start }}
        </button>
        <button
          v-if="model.canStop(row, 'resume-competition')"
          type="button"
          @click="model.stopCompetition(row, 'resume-competition')"
        >
          {{ text.resume }}
        </button>
        <button
          v-if="model.canStop(row, 'pause-competition')"
          type="button"
          @click="model.stopCompetition(row, 'pause-competition')"
        >
          {{ text.pause }}
        </button>
        <button
          v-if="model.canStop(row, 'cancel-competition')"
          type="button"
          @click="model.stopCompetition(row, 'cancel-competition')"
        >
          {{ text.cancel }}
        </button>
      </li>
    </ul>
    <nav :aria-label="text.title">
      <button
        type="button"
        :disabled="model.loading.value || model.pageIndex.value === 0"
        @click="model.previous()"
      >
        {{ text.previous }}
      </button>
      <span>{{ text.page }} {{ model.pageIndex.value + 1 }}</span>
      <button
        type="button"
        :disabled="model.loading.value || model.nextAfter.value === null"
        @click="model.next()"
      >
        {{ text.next }}
      </button>
    </nav>
  </section>
</template>

<style scoped>
.controller-competitions { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; }
ul { padding-left:1.5rem; } li { padding:0.25rem 0; } li span { margin-left:0.6rem; opacity:0.8; }
nav { display:flex; gap:0.7rem; align-items:center; flex-wrap:wrap; }
</style>
