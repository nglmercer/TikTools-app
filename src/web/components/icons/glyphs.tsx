import type { VNode } from 'vue';
import { SvgIcon, type IconComponent, type IconProps } from './icon-base.tsx';

export function IconTikTok({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <path d="M19.589 6.686a4.793 4.793 0 0 1-3.77-4.245V2h-3.445v13.672a2.896 2.896 0 0 1-2.891 2.891 2.896 2.896 0 0 1-2.892-2.891 2.896 2.896 0 0 1 2.892-2.892c.307 0 .602.05.878.142V9.458a6.32 6.32 0 0 0-.878-.061A6.338 6.338 0 0 0 3 15.736a6.338 6.338 0 0 0 6.338 6.338 6.338 6.338 0 0 0 6.338-6.338V8.674c1.23.882 2.732 1.408 4.355 1.457V6.686h-.442z" />
    </SvgIcon>
  );
}

export function IconChat({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
    </SvgIcon>
  );
}

export function IconGift({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="20 12 20 22 4 22 4 12" />
      <rect x="2" y="7" width="20" height="5" />
      <line x1="12" y1="22" x2="12" y2="7" />
      <path d="M12 7H7.5a2.5 2.5 0 0 1 0-5C11 2 12 7 12 7z" />
      <path d="M12 7h4.5a2.5 2.5 0 0 0 0-5C13 2 12 7 12 7z" />
    </SvgIcon>
  );
}

export function IconHeart({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
    </SvgIcon>
  );
}

export function IconUsers({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
      <path d="M16 3.13a4 4 0 0 1 0 7.75" />
    </SvgIcon>
  );
}

export function IconFollow({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
      <circle cx="9" cy="7" r="4" />
      <line x1="19" y1="8" x2="19" y2="14" />
      <line x1="22" y1="11" x2="16" y2="11" />
    </SvgIcon>
  );
}

export function IconShare({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="18" cy="5" r="3" />
      <circle cx="6" cy="12" r="3" />
      <circle cx="18" cy="19" r="3" />
      <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
      <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
    </SvgIcon>
  );
}

export function IconJoin({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
      <polyline points="10 17 15 12 10 7" />
      <line x1="15" y1="12" x2="3" y2="12" />
    </SvgIcon>
  );
}

export function IconBarChart({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <line x1="12" y1="20" x2="12" y2="10" />
      <line x1="18" y1="20" x2="18" y2="4" />
      <line x1="6" y1="20" x2="6" y2="16" />
    </SvgIcon>
  );
}

export function IconRadio({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="12" cy="12" r="2" />
      <path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49m11.31-2.82a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14" />
    </SvgIcon>
  );
}

export function IconDice({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="2" y="2" width="20" height="20" rx="5" ry="5" />
      <circle cx="8.5" cy="8.5" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="15.5" cy="8.5" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="15.5" cy="15.5" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="8.5" cy="15.5" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.5" fill="currentColor" stroke="none" />
    </SvgIcon>
  );
}

export function IconConnected({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M12 22v-5" />
      <path d="M9 8V2" />
      <path d="M15 8V2" />
      <path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z" />
    </SvgIcon>
  );
}

export function IconDisconnected({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M12 22v-5" />
      <path d="M9 8V2" />
      <path d="M15 8V2" />
      <path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z" />
      <line x1="2" y1="2" x2="22" y2="22" />
    </SvgIcon>
  );
}

export function IconSparkles({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M12 4l1.7 4.8 4.8 1.7-4.8 1.7L12 17l-1.7-4.8L5.5 10.5l4.8-1.7z" />
      <path d="M19 15l.9 2.6 2.6.9-2.6.9L19 22l-.9-2.6-2.6-.9 2.6-.9z" />
      <path d="M5 16l.7 2 2 .7-2 .7L5 21.4l-.7-2-2-.7 2-.7z" />
    </SvgIcon>
  );
}

export function IconSettings({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </SvgIcon>
  );
}

export function IconSearch({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </SvgIcon>
  );
}

export function IconRefresh({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="23 4 23 10 17 10" />
      <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
    </SvgIcon>
  );
}

export function IconPower({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M18.36 6.64a9 9 0 1 1-12.73 0" />
      <line x1="12" y1="2" x2="12" y2="12" />
    </SvgIcon>
  );
}

export function IconTrash({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </SvgIcon>
  );
}

export function IconArrowDown({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <line x1="12" y1="5" x2="12" y2="19" />
      <polyline points="19 12 12 19 5 12" />
    </SvgIcon>
  );
}

export function IconSun({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="12" cy="12" r="5" />
      <line x1="12" y1="1" x2="12" y2="3" />
      <line x1="12" y1="21" x2="12" y2="23" />
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
      <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
      <line x1="1" y1="12" x2="3" y2="12" />
      <line x1="21" y1="12" x2="23" y2="12" />
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
      <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
    </SvgIcon>
  );
}

export function IconMoon({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </SvgIcon>
  );
}

export function IconGlobe({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </SvgIcon>
  );
}

export function IconHttp({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M8 3 4 7l4 4" />
      <path d="M4 7h16" />
      <path d="m16 21 4-4-4-4" />
      <path d="M20 17H4" />
    </SvgIcon>
  );
}

export function IconWebhook({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M9 11H7a5 5 0 0 1 0-10h9a5 5 0 0 1 5 5v2" />
      <line x1="12" y1="8" x2="12" y2="22" />
      <path d="m9 19 3 3 3-3" />
    </SvgIcon>
  );
}

export function IconCode({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </SvgIcon>
  );
}

export function IconJson({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M8 3H7a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2 2 2 0 0 1 2 2v4c0 1.1.9 2 2 2h1" />
      <path d="M16 3h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2 2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-1" />
    </SvgIcon>
  );
}

export function IconTemplate({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="3" y="3" width="18" height="7" rx="1" />
      <rect x="3" y="14" width="9" height="7" rx="1" />
      <rect x="16" y="14" width="5" height="7" rx="1" />
    </SvgIcon>
  );
}

export function IconPoints({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="8" cy="8" r="6" />
      <path d="M18.09 10.37A6 6 0 1 1 10.34 18" />
      <path d="M7 6h2v4H7z" fill="currentColor" stroke="none" opacity="0.3" />
    </SvgIcon>
  );
}

/** Backwards-compatible alias: the points glyph is the legacy coins icon. */
export const IconCoins: IconComponent = IconPoints;

export function IconTrophy({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
      <path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
      <path d="M4 22h16" />
      <path d="M10 14.66V17c0 .55-.45 1-1 1H7v4h10v-4h-2c-.55 0-1-.45-1-1v-2.34" />
      <path d="M6 4h12v7a6 6 0 0 1-12 0V4z" />
    </SvgIcon>
  );
}

export function IconCrown({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <path d="M2 4l3 12h14l3-12-5 7-5-7-5 7-5-7zm1 14h18v2H3v-2z" />
    </SvgIcon>
  );
}

export function IconSpeaker({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
      <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
    </SvgIcon>
  );
}

export function IconVolume({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
      <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
      <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
    </SvgIcon>
  );
}

export function IconVoice({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
      <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
      <line x1="12" y1="19" x2="12" y2="23" />
      <line x1="8" y1="23" x2="16" y2="23" />
    </SvgIcon>
  );
}

export function IconCheck({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="20 6 9 17 4 12" />
    </SvgIcon>
  );
}

export function IconInfo({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5" />
      <path d="M12 8h.01" />
    </SvgIcon>
  );
}

export function IconWarning({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
      <line x1="12" y1="9" x2="12" y2="13" />
      <line x1="12" y1="17" x2="12.01" y2="17" />
    </SvgIcon>
  );
}

export function IconFlame({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <path d="M12 23c-4.97 0-9-4.03-9-9 0-4.12 3.28-8.73 6.35-12.08a1 1 0 0 1 1.54.14c.94 1.54 2.23 3.65 2.86 5.09.91-1.3 1.25-2.82 1.25-2.82a1 1 0 0 1 1.63-.44c2.94 2.94 4.37 6.13 4.37 10.11 0 4.97-4.03 9-9 9z" />
    </SvgIcon>
  );
}

export function IconStar({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
    </SvgIcon>
  );
}

export function IconBolt({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <path d="M13 2L3 14h7l-1 8 10-12h-7l1-8z" />
    </SvgIcon>
  );
}

export function IconClose({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </SvgIcon>
  );
}

/** Backwards-compatible alias for the close glyph. */
export const IconX: IconComponent = IconClose;

export function IconPause({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <rect x="6" y="4" width="4" height="16" rx="1" />
      <rect x="14" y="4" width="4" height="16" rx="1" />
    </SvgIcon>
  );
}

export function IconStop({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <rect x="6" y="6" width="12" height="12" rx="2" />
    </SvgIcon>
  );
}

export function IconDot({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className} filled>
      <circle cx="12" cy="12" r="8" />
    </SvgIcon>
  );
}

export function IconPlugins({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="3" y="3" width="7" height="7" rx="2" />
      <rect x="14" y="3" width="7" height="7" rx="2" />
      <rect x="3" y="14" width="7" height="7" rx="2" />
      <path d="M17.5 14v7" />
      <path d="M14 17.5h7" />
    </SvgIcon>
  );
}

export function IconEdit({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" />
    </SvgIcon>
  );
}

/** Backwards-compatible alias for the edit glyph. */
export const IconPencil: IconComponent = IconEdit;
export function IconPlay({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polygon points="6 4 20 12 6 20 6 4" fill="currentColor" stroke="none" />
    </SvgIcon>
  );
}

export function IconPlus({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </SvgIcon>
  );
}

export function IconChevronLeft({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="15 18 9 12 15 6" />
    </SvgIcon>
  );
}

export function IconChevronRight({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <polyline points="9 18 15 12 9 6" />
    </SvgIcon>
  );
}

export function IconFormat({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <line x1="21" y1="6" x2="3" y2="6" />
      <line x1="17" y1="10" x2="3" y2="10" />
      <line x1="21" y1="14" x2="3" y2="14" />
      <line x1="17" y1="18" x2="3" y2="18" />
    </SvgIcon>
  );
}

export function IconCopy({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="9" y="9" width="13" height="13" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </SvgIcon>
  );
}

export function IconLink({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
    </SvgIcon>
  );
}

export function IconLock({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="3" y="11" width="18" height="11" rx="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </SvgIcon>
  );
}

export function IconUnlock({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="3" y="11" width="18" height="11" rx="2" />
      <path d="M7 11V7a5 5 0 0 1 9.9-1" />
    </SvgIcon>
  );
}

export function IconKeyboard({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <rect x="2" y="6" width="20" height="12" rx="2" />
      <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M18 14h.01M9 14h6" />
    </SvgIcon>
  );
}

export function IconAudio({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M4 10v4M8 7v10M12 4v16M16 8v8M20 10v4" />
    </SvgIcon>
  );
}

export function IconDoc({ size = 18, strokeWidth = 1.75, className }: IconProps = {}): VNode {
  return (
    <SvgIcon size={size} strokeWidth={strokeWidth} className={className}>
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="9" y1="13" x2="15" y2="13" />
      <line x1="9" y1="17" x2="13" y2="17" />
    </SvgIcon>
  );
}

/**
 * Semantic aliases: one meaning per name so navigation, toolbars, and empty
 * states never reuse the same glyph for different concepts.
 * - nav `connect` tab = live signal (radio); header/card connect action = plug
 * - nav `analytics` = bar chart; `automation` = sparkles; feed `all` = bolt
 */
export const IconConnect: IconComponent = IconConnected;
export const IconDisconnect: IconComponent = IconDisconnected;
export const IconLive: IconComponent = IconRadio;
export const IconAnalytics: IconComponent = IconBarChart;
export const IconAutomation: IconComponent = IconSparkles;
export const IconLikes: IconComponent = IconHeart;
export const IconGifts: IconComponent = IconGift;
export const IconContributors: IconComponent = IconUsers;
