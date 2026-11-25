<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { FeedMotion } from '../lib/feed-motion.ts';
import { useI18n } from 'vue-i18n';
import ViewerAvatar from '@/entities/viewer/ui/ViewerAvatar.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import type { DisplayEvent } from '@/types.ts';
import { cleanHandle, displayName, eventText, formatEventTime, hasRichText } from '../lib/event-text.ts';
import ChatRichText from './ChatRichText.vue';

const props = withDefaults(
  defineProps<{ event: DisplayEvent; images?: boolean; selected?: boolean; motion?: FeedMotion }>(),
  { images: true, selected: false, motion: () => ({}) },
);
const emit = defineEmits<{
  select: [event: DisplayEvent];
  'open-viewer': [viewer: ViewerIdentity];
}>();

const { t, locale } = useI18n();
const giftImageFailed = ref(false);
watch(() => props.event.giftDetails?.imageUrl, () => { giftImageFailed.value = false; });

const handle = computed(() => cleanHandle(props.event.author));
const name = computed(() => displayName(handle.value, props.event.nickname));
const text = computed(() => eventText(props.event, t));
const rich = computed(() => hasRichText(props.event, text.value));
const time = computed(() => formatEventTime(props.event.receivedAt, locale.value));
const giftImage = computed(() =>
  props.event.kind === 'gift' && props.images && !giftImageFailed.value ? props.event.giftDetails?.imageUrl : undefined,
);

function openViewer(): void {
  emit('open-viewer', { uniqueId: handle.value, nickname: props.event.nickname, avatarUrl: props.event.avatarUrl });
}
</script>

<template>
  <article
    v-motion="motion"
    class="event-card"
    :class="[`event-card--${event.kind}`, { 'event-card--selected': selected }]"
    role="listitem"
    :aria-current="selected || undefined"
  >
    <div
      class="event-card__body"
      :class="{ 'event-card__body--gift': event.kind === 'gift' }"
    >
      <div class="event-card__head">
        <button
          class="event-card__author"
          type="button"
          :aria-label="t('feed.openProfile', { viewer: `@${handle}` })"
          @click="openViewer"
        >
          <ViewerAvatar
            :handle="handle"
            :src="images ? event.avatarUrl : undefined"
            :size="22"
          />
          {{ name }}
        </button>
        <time
          class="event-card__time"
          :datetime="new Date(event.receivedAt).toISOString()"
        >{{ time }}</time>
      </div>
      <button
        class="event-card__content"
        type="button"
        :aria-label="t('feed.openDetails', { viewer: name })"
        @click="emit('select', event)"
      >
        <ChatRichText
          v-if="rich"
          :text="text"
          :emotes="event.emotes"
          :images="images"
        />
        <span v-else>{{ text }}</span>
      </button>
      <img
        v-if="giftImage"
        class="event-card__gift"
        :src="giftImage"
        :alt="event.giftDetails?.name"
        decoding="async"
        referrerpolicy="no-referrer"
        @error="giftImageFailed = true"
      >
      <span
        v-else-if="event.kind === 'gift'"
        class="event-card__gift"
        role="img"
        :aria-label="event.giftDetails?.name || 'Gift'"
      >🎁</span>
    </div>
  </article>
</template>

<style scoped>
.event-card {
  --event-accent: var(--color-border-strong);

  position: relative;
  display: flex;
  gap: var(--space-3);
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-left: 3px solid var(--event-accent);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  content-visibility: auto;
  contain-intrinsic-block-size: auto 64px;
}

.event-card--gift { --event-accent: var(--color-chart-2); }
.event-card--like { --event-accent: var(--color-chart-3); }
.event-card--social { --event-accent: var(--color-chart-4); }
.event-card--chat { --event-accent: var(--color-accent); }
.event-card--selected { background: var(--color-bg-selected); border-color: var(--color-border-selected); }

.event-card__body { display: grid; flex: 1; min-width: 0; gap: var(--space-1); }
/* The gift art is its own grid cell on the right, centered across both lines,
   so it stays at 22px instead of making the card taller. */
.event-card__body--gift { grid-template-columns: minmax(0, 1fr) auto; align-items: center; }
.event-card__body--gift .event-card__head { grid-column: 1; grid-row: 1; }
.event-card__body--gift .event-card__content { grid-column: 1; grid-row: 2; }
.event-card__body--gift .event-card__gift { grid-column: 2; grid-row: 1 / 3; align-self: center; }
.event-card__head { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); min-width: 0; }
.event-card__author,
.event-card__content {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.event-card__author { font-size: var(--font-size-sm); font-weight: var(--font-weight-semibold); }
.event-card__author:hover { text-decoration: underline; }
.event-card__content { font-size: var(--font-size-md); overflow-wrap: anywhere; }
.event-card__content:hover { color: var(--color-text-muted); }
.event-card__author:focus-visible,
.event-card__content:focus-visible { box-shadow: var(--focus-ring); }
/* The timestamp is an overlay so showing it never shifts the card contents. */
.event-card__time {
  position: absolute;
  top: var(--space-1);
  right: var(--space-2);
  z-index: 1;
  padding: 1px var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-panel-header);
  color: var(--color-text);
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--duration-fast);
}
.event-card:hover .event-card__time,
.event-card:focus-within .event-card__time { opacity: 1; }
.event-card__gift { flex: none; width: 22px; height: 22px; object-fit: contain; }
@media (prefers-reduced-motion: reduce) {
  .event-card__time { transition: none; }
}
</style>
