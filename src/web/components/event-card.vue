<script lang="tsx">
import type { DisplayEvent } from '../types.ts';
import { t, type Locale } from '../i18n.ts';
import { IconBolt, IconGift, IconHeart } from './icons.vue';
import { UserAvatar } from './user-avatar.vue';

function getAvatarColor(username: string): string {
  const gradients = [
    'linear-gradient(135deg, #ff0050, #00f2fe)',
    'linear-gradient(135deg, #fe2c55, #ff7e40)',
    'linear-gradient(135deg, #69c9d0, #0070f3)',
    'linear-gradient(135deg, #9b51e0, #fe2c55)',
    'linear-gradient(135deg, #ff007a, #7928ca)',
    'linear-gradient(135deg, #00c9ff, #92fe9d)',
    'linear-gradient(135deg, #f857a6, #ff5858)',
    'linear-gradient(135deg, #ff8a00, #e52e71)',
    'linear-gradient(135deg, #06d6a0, #118ab2)',
  ] as const;
  let hash = 0;
  for (let i = 0; i < username.length; i++) {
    hash = username.charCodeAt(i) + ((hash << 5) - hash);
  }
  const idx = Math.abs(hash) % gradients.length;
  return gradients[idx] ?? gradients[0];
}

type EventCardProps = {
  event: DisplayEvent;
  locale?: Locale;
};

function localizedText(event: DisplayEvent, locale: Locale): string {
  const key = event.i18nKey;
  const p = event.i18nParams ?? {};
  if (!key) return event.text;

  switch (key) {
    case 'giftSent':
      return t(locale, 'giftSent', {
        count: p.count ?? event.giftDetails?.count ?? 1,
        giftName: p.giftName ?? event.giftDetails?.name ?? 'Gift',
        diamonds: p.diamonds ?? event.giftDetails?.diamonds ?? 1,
      });
    case 'likeSent': {
      const c = Number(p.count ?? event.likeCount ?? 1);
      return c === 1
        ? t(locale, 'likeSentOne', { count: c })
        : t(locale, 'likeSentMany', { count: c });
    }
    case 'joinedLive':
      return t(locale, 'joinedLive');
    case 'followedCreator':
      return t(locale, 'followedCreator');
    case 'sharedLive':
      return t(locale, 'sharedLive');
    case 'chatMessage':
      return String(p.comment ?? event.text);
    default:
      return event.text;
  }
}

export function EventCard({ event, locale = 'en' }: EventCardProps) {
  const cleanHandle = event.author.replace(/^@+/, '');
  const level = event.level ?? 1;

  const displayName =
    event.nickname && event.nickname !== cleanHandle ? event.nickname : `@${cleanHandle}`;

  const text = localizedText(event, locale);

  const timeLabel = new Date(event.receivedAt).toLocaleTimeString(locale === 'es' ? 'es-ES' : 'en-US', {
    hour: '2-digit',
    minute: '2-digit',
  });

  return (
    <div class={`tiktok-chat-row ${event.kind}`}>
      <UserAvatar
        uniqueId={cleanHandle}
        nickname={event.nickname}
        avatarUrl={event.avatarUrl}
        fallbackStyle={{ background: getAvatarColor(cleanHandle) }}
      />

      {/* Message Content Container */}
      <div class="tt-content-wrap">
        <div class="tt-message-line">
          {/* Level Badge N.º */}
          <span class="tt-badge-level" title={`Level ${level}`}>
            <span class="tt-badge-icon">
              <IconBolt />
            </span>
            <span class="tt-badge-text">{t(locale, 'levelBadge', { level })}</span>
          </span>

          {/* Author handle/nickname */}
          <span class="tt-author" title={`@${cleanHandle}`}>
            {displayName}
          </span>

          {/* Content text depending on event kind */}
          {event.kind === 'chat' ? (
            <span class="tt-chat-text">{text}</span>
          ) : event.kind === 'gift' ? (
            <span class="tt-gift-text">
              {event.giftDetails?.imageUrl ? (
                <img
                  class="tt-gift-img"
                  src={event.giftDetails.imageUrl}
                  alt={event.giftDetails.name}
                  loading="lazy"
                  referrerpolicy="no-referrer"
                  decoding="async"
                  onError={(e) => {
                    const img = e.currentTarget as HTMLImageElement;
                    console.warn('[gift-img-error]', event.giftDetails?.name, img.src.slice(0, 80));
                    img.style.display = 'none';
                    const fallback = img.nextElementSibling as HTMLElement | null;
                    if (fallback) fallback.style.display = 'inline-flex';
                  }}
                  onLoad={() => {
                    // debug: image loaded successfully
                    // console.log('[gift-img-loaded]', event.giftDetails?.name);
                  }}
                />
              ) : null}
              <span
                class="tt-gift-icon"
                style={{ display: event.giftDetails?.imageUrl ? 'none' : 'inline-flex' }}
                title={event.giftDetails?.name || 'Gift'}
              >
                <IconGift />
              </span>{' '}
              {text}
            </span>
          ) : event.kind === 'like' ? (
            <span class="tt-like-text">
              <span style={{ display: 'inline-flex', verticalAlign: 'middle', marginRight: '4px' }}>
                <IconHeart />
              </span>
              {text}
            </span>
          ) : (
            <span class="tt-social-text">{text}</span>
          )}
        </div>
      </div>

      {/* Point Earned Pill or Time */}
      <div class="tt-row-tail">
        {typeof event.pointsDelta === 'number' && event.pointsDelta > 0 ? (
          <span class="tt-points-badge">{t(locale, 'ptsEarned', { amount: event.pointsDelta })}</span>
        ) : (
          <time class="tt-timestamp">{timeLabel}</time>
        )}
      </div>
    </div>
  );
}

export default EventCard;
</script>
