import type { Component } from 'vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconHeart from '@/shared/ui/icon/icons/IconHeart.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import type { DisplayEvent, EventFilter } from '@/types.ts';

/** One recognizable glyph per event type for the live-feed cards. */
export function eventTypeIcon(event: DisplayEvent): Component {
  if (event.kind === 'gift') return IconCoins;
  if (event.kind === 'like') return IconHeart;
  if (event.kind === 'member') return event.i18nKey === 'joinedLive' ? IconUser : IconInfo;
  if (event.kind === 'social') return event.i18nKey === 'sharedLive' ? IconLink : IconHeart;
  return IconChat;
}

/** Icon for each filter option of the live feed select. */
export function filterIcon(filter: EventFilter): Component {
  switch (filter) {
    case 'all': return IconRows;
    case 'gift': return IconCoins;
    case 'like': return IconHeart;
    case 'follow':
    case 'join': return IconUser;
    case 'social': return IconLink;
    default: return IconChat;
  }
}
