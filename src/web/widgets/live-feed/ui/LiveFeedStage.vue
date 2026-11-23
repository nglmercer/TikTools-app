<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { isEphemeralEvent, visibleEphemeralEvents } from '@/entities/live-event/lib/event-ephemeral.ts';
import { useFeedMotion } from '@/entities/live-event/lib/feed-motion.ts';
import { cleanHandle, eventText } from '@/entities/live-event/lib/event-text.ts';
import { eventTypeIcon } from '@/entities/live-event/lib/event-type-icon.ts';
import EventCard from '@/entities/live-event/ui/EventCard.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import type { DisplayEvent } from '@/types.ts';

const props = withDefaults(defineProps<{
  events: readonly DisplayEvent[];
  selectedEventId: number | null;
  images?: boolean;
}>(), { images: true });
const emit = defineEmits<{
  select: [event: DisplayEvent];
  'open-viewer': [viewer: ViewerIdentity];
}>();

const { t } = useI18n();
const { row: rowMotion, gift: giftMotion } = useFeedMotion(() => props.events);
const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => { timer = setInterval(() => { now.value = Date.now(); }, 1000); });
onBeforeUnmount(() => { if (timer !== null) clearInterval(timer); });

/** Chat, gifts, follows and shares stay on the wall; reactions fade out. */
const important = computed(() => props.events.filter((event) => !isEphemeralEvent(event)));
// Keep only the newest like/join so the reaction strip stays a single compact line.
const reactions = computed(() => visibleEphemeralEvents(props.events, now.value, 1));

function reactionText(event: DisplayEvent): string {
  const value = eventText(event, (key, params) => String(t(key, params ?? {})));
  return value.length > 48 ? `${value.slice(0, 47)}…` : value;
}
</script>

<template>
  <div class="live-stage">
    <div
      class="live-stage__cards"
      role="list"
    >
      <EventCard
        v-for="event in important"
        :key="event.id"
        :event="event"
        :images="images"
        :selected="event.id === selectedEventId"
        :motion="event.kind === 'gift' ? giftMotion : rowMotion"
        @select="emit('select', $event)"
        @open-viewer="emit('open-viewer', $event)"
      />
      <p
        v-if="!important.length"
        class="live-stage__hint"
      >
        {{ t('feed.stage.reactionsOnly') }}
      </p>
    </div>
    <!-- Reactions append at the end, one per line: new events never push the
         permanent chat cards around. -->
    <TransitionGroup
      v-if="reactions.length"
      name="reaction"
      tag="div"
      class="live-stage__reactions"
      role="status"
      :aria-label="t('feed.stage.reactions')"
    >
      <span
        v-for="reaction in reactions"
        :key="reaction.id"
        class="live-stage__reaction"
        :class="`live-stage__reaction--${reaction.kind}`"
      >
        <component
          :is="eventTypeIcon(reaction)"
          :size="12"
        />
        <strong>{{ cleanHandle(reaction.author) }}</strong>
        <span>{{ reactionText(reaction) }}</span>
      </span>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.live-stage { display: flex; flex-direction: column; min-height: 100%; }
.live-stage__reactions {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
}
.live-stage__reaction {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  min-width: 0;
  padding: var(--space-2) var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}
.live-stage__reaction strong { color: var(--color-text); font-weight: var(--font-weight-semibold); }
.live-stage__reaction span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.live-stage__reaction--like { color: var(--color-chart-3); }
.live-stage__reaction--member { color: var(--color-chart-4); }
.reaction-enter-active, .reaction-leave-active { transition: opacity var(--duration-normal), transform var(--duration-normal); }
.reaction-enter-from, .reaction-leave-to { opacity: 0; transform: translateY(-6px); }
.reaction-leave-active { position: absolute; }
.live-stage__cards { display: grid; align-content: start; gap: var(--space-2); padding: var(--space-3) var(--space-4); }
.live-stage__hint { margin: 0; color: var(--color-text-subtle); font-size: var(--font-size-sm); text-align: center; }
@media (prefers-reduced-motion: reduce) {
  .reaction-enter-active, .reaction-leave-active { transition: none; }
}
</style>
