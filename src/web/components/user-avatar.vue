<script lang="tsx">
type UserAvatarProps = {
  uniqueId: string;
  nickname?: string;
  avatarUrl?: string | null;
  imgClass?: string;
  fallbackClass?: string;
  fallbackStyle?: Record<string, string | number>;
};

/**
 * One avatar renderer for feed cards, tops, and tables: the TikTok user
 * image when the backend carried one, initials otherwise. A broken image
 * swaps to the same initials fallback instead of vanishing.
 *
 * `referrerpolicy="no-referrer"` matters: TikTok CDN hosts reject
 * hotlinked avatars that arrive with a foreign `Referer`.
 */
export function UserAvatar({
  uniqueId,
  nickname,
  avatarUrl,
  imgClass = 'tt-avatar-img',
  fallbackClass = 'tt-avatar',
  fallbackStyle,
}: UserAvatarProps) {
  const clean = uniqueId.replace(/^@+/, '');
  const label = nickname && nickname !== clean ? nickname : `@${clean}`;
  const initials = clean.slice(0, 2).toUpperCase() || '•';
  const hasAvatar = Boolean(avatarUrl);

  return (
    <>
      {hasAvatar ? (
        <img
          class={imgClass}
          src={avatarUrl as string}
          alt={label}
          loading="lazy"
          decoding="async"
          referrerpolicy="no-referrer"
          onError={(e) => {
            const img = e.currentTarget as HTMLImageElement;
            img.style.display = 'none';
            const fallback = img.nextElementSibling as HTMLElement | null;
            // Clear the inline hide so the fallback class default applies
            // (`flex` for cards, `inline-flex` for chips).
            if (fallback) fallback.style.display = '';
          }}
        />
      ) : null}
      <span
        class={fallbackClass}
        style={{ ...fallbackStyle, display: hasAvatar ? 'none' : undefined }}
        aria-hidden={hasAvatar}
      >
        {initials}
      </span>
    </>
  );
}

export default UserAvatar;
</script>
