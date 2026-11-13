<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import ViewerAvatar from '@/entities/viewer/ui/ViewerAvatar.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import type { DisplayEvent } from '@/types.ts';
import { cleanHandle, displayName, eventText, formatEventTime, hasRichText } from '../lib/event-text.ts';
import ChatRichText from './ChatRichText.vue';

const props = withDefaults(
  defineProps<{
    event: DisplayEvent;
    images?: boolean;
    selected?: boolean;
  }>(),
  { images: true, selected: false },
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
  <div
    class="event-row"
    :class="[`event-row--${event.kind}`, { 'event-row--selected': selected }]"
    role="listitem"
    :aria-current="selected || undefined"
  >
    <button
      class="event-row__author"
      type="button"
      :title="`@${handle}`"
      :aria-label="t('feed.openProfile', { viewer: `@${handle}` })"
      @click="openViewer"
    >
      <ViewerAvatar
        :handle="handle"
        :src="images ? event.avatarUrl : undefined"
        :size="20"
      />
      <span class="event-row__name">{{ name }}</span>
      <small
        v-if="event.platform && event.platform !== 'tiktok'"
        class="event-row__platform"
      >{{ event.platform }}</small>
    </button>
    <button
      class="event-row__content"
      type="button"
      :title="text"
      :aria-label="t('feed.openDetails', { viewer: name })"
      @click="emit('select', event)"
    >
      <img
        v-if="giftImage"
        class="event-row__gift"
        :src="giftImage"
        :alt="event.giftDetails?.name"
        loading="lazy"
        decoding="async"
        referrerpolicy="no-referrer"
        @error="giftImageFailed = true"
      >
      <span
        v-else-if="event.kind === 'gift'"
        class="event-row__gift"
        role="img"
        :aria-label="event.giftDetails?.name || 'Gift'"
      >🎁</span>
      <span class="event-row__text">
        <ChatRichText
          v-if="rich"
          :text="text"
          :emotes="event.emotes"
          :images="images"
        />
        <template v-else>{{ text }}</template>
      </span>
    </button>
    <time
      class="event-row__time"
      :datetime="new Date(event.receivedAt).toISOString()"
    >{{ time }}</time>
  </div>
</template>

<style scoped>
.event-row {
  --event-accent: transparent;

  display: grid;
  grid-template-columns: minmax(0, max-content) minmax(0, 1fr) 40px;
  gap: var(--space-4);
  align-items: center;
  min-height: 28px;
  padding: var(--space-1) var(--space-4) var(--space-1) var(--space-3);
  border-left: 2px solid var(--event-accent);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.event-row:hover {
  background: var(--color-bg-hover);
}

.event-row--gift {
  --event-accent: var(--color-chart-2);
}

.event-row--like {
  --event-accent: var(--color-chart-3);
}

.event-row--social {
  --event-accent: var(--color-chart-4);
}

.event-row--member {
  --event-accent: var(--color-border-strong);
}

.event-row--selected,
.event-row--selected:hover {
  background: var(--color-bg-selected);
}

.event-row__time {
  justify-self: end;
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
  opacity: 0.45;
  transition: opacity var(--duration-fast);
}

.event-row:hover .event-row__time,
.event-row:focus-within .event-row__time {
  opacity: 1;
}

@media (prefers-reduced-motion: reduce) {
  .event-row__time {
    transition: none;
  }
}

.event-row__author,
.event-row__content {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-3);
  min-width: 0;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.event-row__author:focus-visible,
.event-row__content:focus-visible {
  box-shadow: var(--focus-ring);
}

.event-row__name {
  max-width: 180px;
  overflow: hidden;
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.event-row__author:hover .event-row__name {
  text-decoration: underline;
}

.event-row__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.event-row--member .event-row__text,
.event-row--social .event-row__text,
.event-row--like .event-row__text {
  color: var(--color-text-muted);
}

.event-row__gift {
  flex: none;
  width: 20px;
  height: 20px;
  object-fit: contain;
}
</style>
