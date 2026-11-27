import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import type { DisplayEvent } from '@/types.ts';
import type { FeedMotion } from '../lib/feed-motion.ts';
import { cleanHandle, displayName, eventText, formatEventTime, hasRichText } from '../lib/event-text.ts';

/** What a feed entry needs, whether it is drawn as a card or as a compact row. */
export interface EventViewProps {
  event: DisplayEvent;
  images?: boolean;
  selected?: boolean;
  motion?: FeedMotion;
}

export const eventViewDefaults = { images: true, selected: false, motion: () => ({}) };

/**
 * The text, time, avatar handle and gift image of a feed event. `EventCard` and `EventRow` differ only in markup, so
 * the derivation lives here once instead of being kept in step in both components.
 */
export function useEventDisplay(props: Readonly<EventViewProps>, onOpenViewer: (viewer: ViewerIdentity) => void) {
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

  return {
    t,
    handle,
    name,
    text,
    rich,
    time,
    giftImage,
    markGiftImageFailed: () => { giftImageFailed.value = true; },
    openViewer: () => onOpenViewer({ uniqueId: handle.value, nickname: props.event.nickname, avatarUrl: props.event.avatarUrl }),
  };
}
