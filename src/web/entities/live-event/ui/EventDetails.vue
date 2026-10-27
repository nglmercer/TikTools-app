<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import ViewerAvatar from '@/entities/viewer/ui/ViewerAvatar.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import type { DisplayEvent } from '@/types.ts';
import { cleanHandle, displayName, eventText, formatEventStamp, hasRichText } from '../lib/event-text.ts';
import ChatRichText from './ChatRichText.vue';

const props = withDefaults(
  defineProps<{
    event: DisplayEvent;
    images?: boolean;
  }>(),
  { images: true },
);

const emit = defineEmits<{ 'open-viewer': [viewer: ViewerIdentity] }>();
const { t, locale } = useI18n();

const handle = computed(() => cleanHandle(props.event.author));
const name = computed(() => displayName(handle.value, props.event.nickname));
const text = computed(() => eventText(props.event, t));
const rich = computed(() => hasRichText(props.event, text.value));

const rows = computed(() => {
  const event = props.event;
  const value = (number: number | undefined): string =>
    typeof number === 'number' && Number.isFinite(number) ? formatCount(number, locale.value) : t('feed.details.unavailable');
  const entries: Array<[string, string]> = [['kind', t(`feed.kinds.${event.kind}`)]];
  if (event.kind === 'gift') {
    if (event.giftDetails?.name) entries.push(['giftName', event.giftDetails.name]);
    entries.push(['giftCount', value(event.giftDetails?.count)], ['giftDiamonds', value(event.giftDetails?.diamonds)]);
  }
  if (event.kind === 'like') {
    entries.push(['likeCount', value(event.likeCount)]);
    if (event.likeUserTotal !== undefined) {
      entries.push(['likeUserTotal', value(event.likeUserTotal)]);
    }
  }
  if (typeof event.level === 'number') entries.push(['level', value(event.level)]);
  entries.push([
    'points',
    typeof event.pointsDelta === 'number' ? `+${formatCount(event.pointsDelta, locale.value)}` : t('feed.details.unavailable'),
  ]);
  entries.push(['time', formatEventStamp(event.receivedAt, locale.value)]);
  if (event.msgId) entries.push(['messageId', event.msgId]);
  if (rich.value) entries.push(['rawComment', text.value]);
  return entries.map(([key, content]) => ({ key, label: t(`feed.details.${key}`), content }));
});

function openViewer(): void {
  emit('open-viewer', { uniqueId: handle.value, nickname: props.event.nickname, avatarUrl: props.event.avatarUrl });
}
</script>

<template>
  <div class="event-details">
    <div class="event-details__identity">
      <ViewerAvatar
        :handle="handle"
        :src="images ? event.avatarUrl : undefined"
        :size="36"
      />
      <span class="event-details__names">
        <strong>{{ name }}</strong>
        <span class="event-details__handle">@{{ handle }}</span>
      </span>
    </div>
    <p class="event-details__text">
      <ChatRichText
        v-if="rich"
        :text="text"
        :emotes="event.emotes"
        :images="images"
      />
      <template v-else>
        {{ text }}
      </template>
    </p>
    <dl class="event-details__rows">
      <div
        v-for="row in rows"
        :key="row.key"
        class="event-details__row"
      >
        <dt>{{ row.label }}</dt>
        <dd :title="row.content">
          {{ row.content }}
        </dd>
      </div>
    </dl>
    <BaseButton
      block
      @click="openViewer"
    >
      {{ t('feed.inspector.profile') }}
    </BaseButton>
  </div>
</template>

<style scoped>
.event-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.event-details__identity {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.event-details__names {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.event-details__handle {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.event-details__text {
  margin: 0;
  padding: var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
  line-height: var(--line-height-normal);
  overflow-wrap: anywhere;
}

.event-details__rows {
  margin: 0;
}

.event-details__row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-5);
  padding: var(--space-3) 0;
}

.event-details__row + .event-details__row {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.event-details__row dt {
  flex: none;
  color: var(--color-text-muted);
}

.event-details__row dd {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
