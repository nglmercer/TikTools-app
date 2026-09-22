/**
 * Single configuration source for the TikTools alert widgets.
 *
 * The widgets are renderers of the canonical TikTools event stream: they
 * connect to the loopback event gateway, never to TikTok directly.
 */

export const DEFAULT_GATEWAY_HOST = '127.0.0.1';
export const DEFAULT_GATEWAY_PORT = 17452;

/** Stable transport topic carrying normalized TikTok automation events. */
export const LIVE_EVENT_TOPIC = 'live.event';
/** Transport topic signalling skipped broadcast messages. Never replayed. */
export const EVENT_GAP_TOPIC = 'event.gap';

export const DEFAULT_TOPICS: readonly string[] = [LIVE_EVENT_TOPIC, EVENT_GAP_TOPIC];

export const FOLLOW_EVENT_TYPE = 'tiktok.follow';
export const GIFT_EVENT_TYPE = 'tiktok.gift';
export const CHAT_EVENT_TYPE = 'tiktok.chat';
export const SHARE_EVENT_TYPE = 'tiktok.share';
export const JOIN_EVENT_TYPE = 'tiktok.join';

/**
 * Proto `MemberMessageAction` for subscriptions: TikTok delivers a new
 * subscriber as a member event with `action == 3`, which the host forwards
 * as `tiktok.join`. The subscribe widget keys off this value.
 */
export const SUBSCRIBE_MEMBER_ACTION = 3;

/** Reconnect backoff: 500ms doubling to a 10s ceiling. */
export const RECONNECT_BASE_MS = 500;
export const RECONNECT_MAX_MS = 10_000;
export const RECONNECT_FACTOR = 2;
export const RECONNECT_JITTER_MS = 250;

/** Gateway JSON heartbeat cadence (`{"type":"ping"}` -> `{"type":"pong"}`). */
export const HEARTBEAT_INTERVAL_MS = 20_000;
/** Reconnect when no frame at all arrives for this many heartbeats. */
export const HEARTBEAT_MISSED_LIMIT = 3;

/** Bounded alert queue capacity. Documented policy: drop-oldest. */
export const DEFAULT_QUEUE_CAPACITY = 100;
/** Bounded duplicate-protection cache for recently seen event ids. */
export const DEFAULT_RECENT_IDS_CAPACITY = 1000;

export const DEFAULT_FOLLOW_VISIBLE_MS = 4000;
export const DEFAULT_FOLLOW_ENTER_MS = 400;
export const DEFAULT_FOLLOW_EXIT_MS = 400;

export const DEFAULT_SHARE_VISIBLE_MS = 4000;
export const DEFAULT_SHARE_ENTER_MS = 400;
export const DEFAULT_SHARE_EXIT_MS = 400;

export const DEFAULT_SUBSCRIBE_VISIBLE_MS = 5000;
export const DEFAULT_SUBSCRIBE_ENTER_MS = 400;
export const DEFAULT_SUBSCRIBE_EXIT_MS = 400;

export const DEFAULT_CHAT_MESSAGE_LIMIT = 30;

export const DEFAULT_GIFT_VISIBLE_MS = 5000;
export const DEFAULT_GIFT_COMBO_TIMEOUT_MS = 3000;
export const DEFAULT_GIFT_MINIMUM_DIAMONDS = 0;

/**
 * Widget-scoped WebSocket endpoint. `/ws/widgets` honors only the widget
 * credential and only widget topics; the full `/ws` surface stays
 * exclusive to external gateway clients.
 */
export function gatewayWsUrl(host: string, port: number): string {
  return `ws://${host}:${port}/ws/widgets`;
}

export function gatewayHealthUrl(host: string, port: number): string {
  return `http://${host}:${port}/health`;
}

export function gatewayWidgetUrl(host: string, port: number, widget: 'follow' | 'gift'): string {
  return `http://${host}:${port}/widgets/${widget}/`;
}

function readPositiveInt(params: URLSearchParams, name: string, fallback: number): number {
  const raw = params.get(name);
  if (raw === null || raw.trim() === '') return fallback;
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value) || value < 0) return fallback;
  return value;
}

function readBoolean(params: URLSearchParams, name: string, fallback: boolean): boolean {
  const raw = params.get(name);
  if (raw === null || raw.trim() === '') return fallback;
  if (raw === '1' || raw.toLowerCase() === 'true') return true;
  if (raw === '0' || raw.toLowerCase() === 'false') return false;
  return fallback;
}

export interface FollowWidgetSettings {
  visibleMs: number;
  enterMs: number;
  exitMs: number;
  showUniqueId: boolean;
}

export function parseFollowSettings(search: string): FollowWidgetSettings {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  return {
    visibleMs: readPositiveInt(params, 'duration', DEFAULT_FOLLOW_VISIBLE_MS),
    enterMs: readPositiveInt(params, 'enter', DEFAULT_FOLLOW_ENTER_MS),
    exitMs: readPositiveInt(params, 'exit', DEFAULT_FOLLOW_EXIT_MS),
    showUniqueId: readBoolean(params, 'handle', true),
  };
}

/**
 * Preview mode from the URL fragment (`#demo=follow|gift|combo|chat|share|
 * subscribe`). The widget injects synthetic content locally after mount — no
 * gateway connection is attempted for the demo. Used by the TikTools preview
 * iframes.
 */
export type WidgetDemoMode = 'follow' | 'gift' | 'combo' | 'chat' | 'share' | 'subscribe';

export function parseDemoMode(hash: string): WidgetDemoMode | null {
  const params = new URLSearchParams(hash.startsWith('#') ? hash.slice(1) : hash);
  const demo = params.get('demo');
  return demo === 'follow' ||
    demo === 'gift' ||
    demo === 'combo' ||
    demo === 'chat' ||
    demo === 'share' ||
    demo === 'subscribe'
    ? demo
    : null;
}

export interface ShareWidgetSettings {
  visibleMs: number;
  enterMs: number;
  exitMs: number;
  showUniqueId: boolean;
}

export function parseShareSettings(search: string): ShareWidgetSettings {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  return {
    visibleMs: readPositiveInt(params, 'duration', DEFAULT_SHARE_VISIBLE_MS),
    enterMs: readPositiveInt(params, 'enter', DEFAULT_SHARE_ENTER_MS),
    exitMs: readPositiveInt(params, 'exit', DEFAULT_SHARE_EXIT_MS),
    showUniqueId: readBoolean(params, 'handle', true),
  };
}

export interface SubscribeWidgetSettings {
  visibleMs: number;
  enterMs: number;
  exitMs: number;
  showUniqueId: boolean;
}

export function parseSubscribeSettings(search: string): SubscribeWidgetSettings {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  return {
    visibleMs: readPositiveInt(params, 'duration', DEFAULT_SUBSCRIBE_VISIBLE_MS),
    enterMs: readPositiveInt(params, 'enter', DEFAULT_SUBSCRIBE_ENTER_MS),
    exitMs: readPositiveInt(params, 'exit', DEFAULT_SUBSCRIBE_EXIT_MS),
    showUniqueId: readBoolean(params, 'handle', true),
  };
}

export interface ChatWidgetSettings {
  limit: number;
  showAvatars: boolean;
}

export function parseChatSettings(search: string): ChatWidgetSettings {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  return {
    limit: Math.max(1, readPositiveInt(params, 'limit', DEFAULT_CHAT_MESSAGE_LIMIT)),
    showAvatars: readBoolean(params, 'avatars', true),
  };
}

export interface GiftWidgetSettings {
  visibleMs: number;
  comboTimeoutMs: number;
  minimumDiamonds: number;
  showImage: boolean;
  showDiamonds: boolean;
  showCount: boolean;
}

export function parseGiftSettings(search: string): GiftWidgetSettings {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  return {
    visibleMs: readPositiveInt(params, 'duration', DEFAULT_GIFT_VISIBLE_MS),
    comboTimeoutMs: readPositiveInt(params, 'comboTimeout', DEFAULT_GIFT_COMBO_TIMEOUT_MS),
    minimumDiamonds: readPositiveInt(params, 'minDiamonds', DEFAULT_GIFT_MINIMUM_DIAMONDS),
    showImage: readBoolean(params, 'image', true),
    showDiamonds: readBoolean(params, 'diamonds', true),
    showCount: readBoolean(params, 'count', true),
  };
}
