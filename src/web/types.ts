import type {
  CreatorRecord,
  LikeLeaderPayload,
  PointsConfig,
  TopViewerPayload,
  UiEvent,
  ViewerRecord,
} from '../shared/messages.ts';
import type { JsonObject } from '../automation/types.ts';

export type BuiltinAppTab = 'feed' | 'points' | 'analytics' | 'connect' | 'behavior' | 'plugins' | 'widgets' | 'settings';

/** Builtin tabs plus plugin page tabs (`plugin:<pluginId>:<pageId>`). */
export type AppTab = BuiltinAppTab | `plugin:${string}:${string}`;

export type ConnectionStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'retrying'
  | 'disconnected'
  | 'error';

export type DisplayEvent = UiEvent & {
  id: number;
  receivedAt: number;
  /** Host feed journal order, independent of arrival order during backfill. */
  feedCursor?: number;
};

export type EventFilter = 'all' | 'chat' | 'gift' | 'like' | 'social' | 'follow' | 'join';

/**
 * Which contextual panel is showing beside the feed. `event`/`user`
 * selections carry the session that owns them so a creator change can
 * clear a stale selection instead of resolving it against the new feed.
 */
export type FeedAsideState =
  | { kind: 'closed' }
  | { kind: 'event'; sessionId: string; eventId: number }
  | { kind: 'user'; sessionId: string; uniqueId: string }
  | { kind: 'viewers' };

/**
 * A currently-live room from the read-only `live.channels` browse.
 * `roomId` stays a string: TikTok ids exceed the JS safe integer range.
 */
export type LiveChannel = {
  uniqueId: string;
  roomId: string;
  nickname: string;
  title: string;
  viewers: number;
};

/**
 * Enriched viewer profile for the user card. Feed events carry
 * only identity fields; the leaderboard (when the viewer is
 * tracked) fills in the engagement stats.
 */
export type UserCardUser = {
  uniqueId: string;
  nickname?: string;
  avatarUrl?: string;
  points?: number;
  level?: number;
  isSubscriber?: boolean;
  totalChats?: number;
  totalCoins?: number;
  totalLikes?: number;
  totalShares?: number;
  firstSeen?: number;
  lastSeen?: number;
};

export type PluginSettingsState = {
  schema: JsonObject;
  uiHints?: JsonObject;
  values: JsonObject;
};

export type { CreatorRecord, LikeLeaderPayload, PointsConfig, TopViewerPayload, ViewerRecord };
