<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { LiveFeedEvent, LiveFeedModel } from '../model/live-feed.ts';

const props = defineProps<{ model: LiveFeedModel; canLookupPoints?: (event: LiveFeedEvent) => boolean; showAmbiguousViewer?: boolean }>();
const emit = defineEmits<{ lookupPoints: [event: LiveFeedEvent] }>();
const { t, tm } = useI18n();
const text = computed(() => tm('controller.liveFeed.text') as MessageSchema['controller']['liveFeed']['text']);
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
  <section
    v-if="model.allowed.value"
    class="live-feed"
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
      v-else-if="model.gap.value"
      role="status"
    >
      {{ text.gap }}
    </p>
    <div class="filters">
      <label v-if="model.creatorOptions.value.length">{{ text.creator }}
        <select
          :value="model.selectedCreator.value ?? ''"
          @change="model.selectCreator(($event.target as HTMLSelectElement).value)"
        >
          <option
            v-for="creator in model.creatorOptions.value"
            :key="creator"
            :value="creator"
          >{{ creator }}</option>
        </select>
      </label>
      <template v-if="!model.error.value">
        <label>{{ text.kind }}
          <select v-model="selectedKind">
            <option value="all">{{ text.all }}</option>
            <option
              v-for="(label, kind) in text.kinds"
              :key="kind"
              :value="kind"
            >{{ label }}</option>
          </select>
        </label>
        <label>{{ text.search }}
          <input
            v-model="query"
            type="search"
            maxlength="128"
            autocomplete="off"
          >
        </label>
      </template>
    </div>
    <template v-if="!model.error.value">
      <p v-if="model.events.value.length === 0">
        {{ text.empty }}
      </p>
      <p v-else-if="visibleEvents.length === 0">
        {{ text.noMatches }}
      </p>
      <ol
        v-else
        role="log"
        :aria-label="text.title"
      >
        <li
          v-for="event in visibleEvents"
          :key="event.cursor"
        >
          <strong>{{ event.nickname || event.author }}</strong>
          <span class="kind">{{ (text.kinds as Record<string, string>)[event.kind] }}</span>
          <span>{{ event.text }}</span>
          <button
            v-if="event.authorLookupId && canLookupPoints?.(event)"
            type="button"
            @click="emit('lookupPoints', event)"
          >
            {{ t('controller.liveFeed.text.pointsFor', { id: event.authorLookupId }) }}
          </button>
          <span
            v-else-if="showAmbiguousViewer && event.authorLookupId === null"
            class="ambiguous"
          >{{ text.ambiguousViewer }}</span>
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
