import { intlLocale } from '@/shared/lib/intl.ts';
import type { DisplayEvent } from '@/types.ts';

type Translate = (key: string, params?: Record<string, unknown>) => string;

export function eventText(event: DisplayEvent, t: Translate): string {
  const params = event.i18nParams ?? {};
  switch (event.i18nKey) {
    case 'giftSent':
      return t('feed.events.giftSent', {
        count: params.count ?? event.giftDetails?.count ?? 1,
        giftName: params.giftName ?? event.giftDetails?.name ?? 'Gift',
        diamonds: params.diamonds ?? event.giftDetails?.diamonds ?? 1,
      });
    case 'likeSent': {
      // The backend counter describes likes observed from this viewer during
      // this LIVE, not the incoming packet's (unchanged) likeCount.
      const total = event.likeUserTotal;
      if (typeof total === 'number' && Number.isSafeInteger(total) && total > 0) {
        return t('feed.events.likeSentTotal', { count: total });
      }
      const count = Number(params.count ?? event.likeCount ?? 1);
      return t(count === 1 ? 'feed.events.likeSentOne' : 'feed.events.likeSentMany', { count });
    }
    case 'joinedLive':
      return t('feed.events.joined');
    case 'followedCreator':
      return t('feed.events.followed');
    case 'sharedLive':
      return t('feed.events.shared');
    case 'chatMessage':
      return String(params.comment ?? event.text);
    default:
      return event.text;
  }
}

export function hasRichText(event: DisplayEvent, text: string): boolean {
  return event.kind === 'chat' && ((event.emotes?.length ?? 0) > 0 || text.includes('['));
}

export function cleanHandle(author: string): string {
  return author.replace(/^@+/, '');
}

export function displayName(handle: string, nickname?: string): string {
  return nickname && nickname !== handle ? nickname : `@${handle}`;
}

const timeFormatters = new Map<string, Intl.DateTimeFormat>();

export function formatEventTime(value: number, locale: string): string {
  let formatter = timeFormatters.get(locale);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(intlLocale(locale), { hour: '2-digit', minute: '2-digit' });
    timeFormatters.set(locale, formatter);
  }
  return formatter.format(value);
}

export function formatEventStamp(value: number, locale: string): string {
  return new Date(value).toLocaleString(intlLocale(locale), {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}
