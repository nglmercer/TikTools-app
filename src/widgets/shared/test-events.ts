/**
 * Development-only fake event factories plus the page test hook. Production
 * widget behavior never depends on this API; the hook exists so tests and
 * manual previews can inject envelopes without a TikTok connection.
 */

import type { JsonValue } from '../../shared/json.ts';
import { EVENT_GAP_TOPIC, LIVE_EVENT_TOPIC, SUBSCRIBE_MEMBER_ACTION } from './config.ts';
import type {
  AutomationEventLike,
  AutomationUser,
  ChatAutomationEvent,
  ConnectionAutomationEvent,
  DomainEventEnvelope,
  FollowAutomationEvent,
  GiftAutomationEvent,
  JoinAutomationEvent,
  LikeAutomationEvent,
  PluginEmitAutomationEvent,
  PointsAwardedAutomationEvent,
  RoomStatsAutomationEvent,
  ShareAutomationEvent,
  SocialAutomationEvent,
} from './event-types.ts';

export const WIDGET_TEST_HOOK = '__tiktoolsWidgetTest';

let sequence = 0;

function nextId(prefix: string): string {
  sequence += 1;
  return `${prefix}-test-${sequence}`;
}

export interface TestUserOverrides {
  userId?: string;
  uniqueId?: string;
  nickname?: string;
  secUid?: string;
  avatarUrl?: string | null;
}

export function makeTestUser(overrides: TestUserOverrides = {}): AutomationUser {
  return {
    userId: overrides.userId ?? '123',
    uniqueId: overrides.uniqueId ?? 'viewer_name',
    nickname: overrides.nickname ?? 'Viewer Name',
    secUid: overrides.secUid ?? 'sec-test',
    avatarUrl: overrides.avatarUrl ?? null,
  };
}

export interface TestFollowOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  action?: number;
  isHistory?: boolean;
}

export function makeTestFollowEvent(overrides: TestFollowOverrides = {}): FollowAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-follow'),
    type: 'tiktok.follow',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      action: overrides.action ?? 1,
      followCount: 42,
      shareCount: 0,
      method: 'test',
      msgId: 'msg-follow',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export interface TestGiftOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  giftId?: string;
  giftName?: string;
  diamondCount?: number;
  repeatCount?: number;
  comboCount?: number;
  groupId?: string;
  repeatEnd?: boolean;
  streakable?: boolean;
  giftIconUrl?: string | null;
  isHistory?: boolean;
}

export function makeTestGiftEvent(overrides: TestGiftOverrides = {}): GiftAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-gift'),
    type: 'tiktok.gift',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      giftId: overrides.giftId ?? 'gift-galaxy',
      giftName: overrides.giftName ?? 'Galaxy',
      diamondCount: overrides.diamondCount ?? 1000,
      repeatCount: overrides.repeatCount ?? 1,
      comboCount: overrides.comboCount ?? 1,
      groupId: overrides.groupId ?? `group-${sequence}`,
      repeatEnd: overrides.repeatEnd ?? false,
      streakable: overrides.streakable ?? false,
      giftIconUrl: overrides.giftIconUrl ?? null,
      method: 'test',
      msgId: 'msg-gift',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export interface TestChatOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  comment?: string;
  isHistory?: boolean;
}

export function makeTestChatEvent(overrides: TestChatOverrides = {}): ChatAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-chat'),
    type: 'tiktok.chat',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      comment: overrides.comment ?? 'Hello from the test chat!',
      method: 'test',
      msgId: 'msg-chat',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export interface TestShareOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  action?: number;
  isHistory?: boolean;
}

export function makeTestShareEvent(overrides: TestShareOverrides = {}): ShareAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-share'),
    type: 'tiktok.share',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      action: overrides.action ?? 3,
      followCount: 0,
      shareCount: 7,
      method: 'test',
      msgId: 'msg-share',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export interface TestSubscribeOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  action?: number;
  memberCount?: number;
  isHistory?: boolean;
}

export function makeTestSubscribeEvent(overrides: TestSubscribeOverrides = {}): JoinAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-subscribe'),
    type: 'tiktok.join',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      memberCount: overrides.memberCount ?? 128,
      action: overrides.action ?? SUBSCRIBE_MEMBER_ACTION,
      method: 'test',
      msgId: 'msg-subscribe',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export function toLiveEnvelope(event: AutomationEventLike): DomainEventEnvelope {
  return { topic: LIVE_EVENT_TOPIC, data: { eventType: event.type, event } };
}

export function makeTestFollowEnvelope(overrides?: TestFollowOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestFollowEvent(overrides));
}

export function makeTestGiftEnvelope(overrides?: TestGiftOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestGiftEvent(overrides));
}

export function makeTestChatEnvelope(overrides?: TestChatOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestChatEvent(overrides));
}

export function makeTestShareEnvelope(overrides?: TestShareOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestShareEvent(overrides));
}

export function makeTestSubscribeEnvelope(overrides?: TestSubscribeOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestSubscribeEvent(overrides));
}

export interface TestLikeOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  count?: number;
  total?: number;
  isHistory?: boolean;
}

export function makeTestLikeEvent(overrides: TestLikeOverrides = {}): LikeAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-like'),
    type: 'tiktok.like',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      count: overrides.count ?? 5,
      total: overrides.total ?? 128,
      method: 'test',
      msgId: 'msg-like',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export function makeTestLikeEnvelope(overrides?: TestLikeOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestLikeEvent(overrides));
}

export interface TestSocialOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  action?: number;
  isHistory?: boolean;
}

export function makeTestSocialEvent(overrides: TestSocialOverrides = {}): SocialAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-social'),
    type: 'tiktok.social',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      action: overrides.action ?? 0,
      followCount: 1,
      shareCount: 1,
      method: 'test',
      msgId: 'msg-social',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export function makeTestSocialEnvelope(overrides?: TestSocialOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestSocialEvent(overrides));
}

export interface TestJoinOverrides {
  id?: string;
  user?: TestUserOverrides | null;
  memberCount?: number;
  isHistory?: boolean;
}

/** Plain room join (member action 0); subscribes use {@link makeTestSubscribeEvent}. */
export function makeTestJoinEvent(overrides: TestJoinOverrides = {}): JoinAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-join'),
    type: 'tiktok.join',
    timestamp: Date.now(),
    user: overrides.user === null ? null : makeTestUser(overrides.user),
    data: {
      memberCount: overrides.memberCount ?? 129,
      action: 0,
      method: 'test',
      msgId: 'msg-join',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export function makeTestJoinEnvelope(overrides?: TestJoinOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestJoinEvent(overrides));
}

export interface TestRoomStatsOverrides {
  id?: string;
  viewers?: number;
  totalUsers?: number;
  popularity?: number;
  anonymous?: number;
  isHistory?: boolean;
}

export function makeTestRoomStatsEvent(overrides: TestRoomStatsOverrides = {}): RoomStatsAutomationEvent {
  return {
    id: overrides.id ?? nextId('tiktok-room-stats'),
    type: 'tiktok.room_stats',
    timestamp: Date.now(),
    data: {
      viewers: overrides.viewers ?? 128,
      totalUsers: overrides.totalUsers ?? 256,
      popularity: overrides.popularity ?? 1024,
      anonymous: overrides.anonymous ?? 4,
      method: 'test',
      msgId: 'msg-room-stats',
      isHistory: overrides.isHistory ?? false,
    },
  };
}

export function makeTestRoomStatsEnvelope(overrides?: TestRoomStatsOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestRoomStatsEvent(overrides));
}

export interface TestConnectionOverrides {
  id?: string;
  uniqueId?: string;
  roomId?: string;
}

export function makeTestConnectionEvent(
  type: 'tiktok.connected' | 'tiktok.disconnected',
  overrides: TestConnectionOverrides = {},
): ConnectionAutomationEvent {
  return {
    id: overrides.id ?? nextId(type.replace('.', '-')),
    type,
    timestamp: Date.now(),
    data: {
      uniqueId: overrides.uniqueId ?? 'creator_demo',
      roomId: overrides.roomId ?? 'room-1',
    },
  };
}

export function makeTestConnectionEnvelope(
  type: 'tiktok.connected' | 'tiktok.disconnected' = 'tiktok.connected',
  overrides?: TestConnectionOverrides,
): DomainEventEnvelope {
  return toLiveEnvelope(makeTestConnectionEvent(type, overrides));
}

export interface TestPointsAwardedOverrides {
  id?: string;
  uniqueId?: string;
  delta?: number;
  totalPoints?: number;
  level?: number;
  currencyName?: string;
  reason?: string;
}

export function makeTestPointsAwardedEvent(
  overrides: TestPointsAwardedOverrides = {},
): PointsAwardedAutomationEvent {
  return {
    id: overrides.id ?? nextId('points-awarded'),
    type: 'points.awarded',
    timestamp: Date.now(),
    data: {
      uniqueId: overrides.uniqueId ?? 'viewer_name',
      delta: overrides.delta ?? 10,
      totalPoints: overrides.totalPoints ?? 110,
      level: overrides.level ?? 2,
      currencyName: overrides.currencyName ?? 'Stars',
      reason: overrides.reason ?? 'gift',
    },
  };
}

export function makeTestPointsAwardedEnvelope(
  overrides?: TestPointsAwardedOverrides,
): DomainEventEnvelope {
  return toLiveEnvelope(makeTestPointsAwardedEvent(overrides));
}

export interface TestPluginEmitOverrides {
  id?: string;
  emitType?: string;
  depth?: number;
  payload?: unknown;
}

export function makeTestPluginEmitEvent(
  overrides: TestPluginEmitOverrides = {},
): PluginEmitAutomationEvent {
  return {
    id: overrides.id ?? nextId('plugin-emit'),
    type: 'plugin.emit',
    timestamp: Date.now(),
    data: {
      emitType: overrides.emitType ?? 'plugin.sample',
      depth: overrides.depth ?? 0,
      payload: (overrides.payload ?? {}) as JsonValue,
    },
  };
}

export function makeTestPluginEmitEnvelope(overrides?: TestPluginEmitOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestPluginEmitEvent(overrides));
}

/** Every built-in trigger the test hook can emulate. */
export const TEST_EMULATABLE_TYPES = [
  'tiktok.chat',
  'tiktok.gift',
  'tiktok.like',
  'tiktok.follow',
  'tiktok.share',
  'tiktok.social',
  'tiktok.join',
  'tiktok.room_stats',
  'tiktok.connected',
  'tiktok.disconnected',
  'points.awarded',
  'plugin.emit',
] as const;

export type TestEmulatableType = (typeof TEST_EMULATABLE_TYPES)[number];

export interface GenericTestOverrides {
  id?: string;
  /** Ignored for envelope-only events (room stats, connection, points, emits). */
  user?: TestUserOverrides | null;
  /** Shallow-merged onto the event `data` payload. */
  data?: Record<string, unknown>;
}

/**
 * One dispatcher for every emulatable trigger, so callers emulate by type
 * name instead of importing a factory per type. Unknown types degrade to a
 * minimal envelope instead of failing.
 */
export function makeTestEnvelopeFor(
  type: string,
  overrides: GenericTestOverrides = {},
): DomainEventEnvelope {
  const identity = { id: overrides.id, user: overrides.user };
  let envelope: DomainEventEnvelope;
  switch (type) {
    case 'tiktok.follow':
      envelope = makeTestFollowEnvelope(identity);
      break;
    case 'tiktok.gift':
      envelope = makeTestGiftEnvelope(identity);
      break;
    case 'tiktok.chat':
      envelope = makeTestChatEnvelope(identity);
      break;
    case 'tiktok.share':
      envelope = makeTestShareEnvelope(identity);
      break;
    case 'tiktok.join':
      envelope = makeTestJoinEnvelope(identity);
      break;
    case 'tiktok.like':
      envelope = makeTestLikeEnvelope(identity);
      break;
    case 'tiktok.social':
      envelope = makeTestSocialEnvelope(identity);
      break;
    case 'tiktok.room_stats':
      envelope = makeTestRoomStatsEnvelope({ id: overrides.id });
      break;
    case 'tiktok.connected':
    case 'tiktok.disconnected':
      envelope = makeTestConnectionEnvelope(type, { id: overrides.id });
      break;
    case 'points.awarded':
      envelope = makeTestPointsAwardedEnvelope({ id: overrides.id });
      break;
    case 'plugin.emit':
      envelope = makeTestPluginEmitEnvelope({ id: overrides.id });
      break;
    default:
      envelope = toLiveEnvelope({
        id: overrides.id ?? nextId('test'),
        type,
        timestamp: Date.now(),
        user: overrides.user === null ? null : makeTestUser(overrides.user),
        data: {},
      });
      break;
  }
  if (overrides.data) {
    const event = (envelope.data as { event: { data: Record<string, unknown> } }).event;
    Object.assign(event.data, overrides.data);
  }
  return envelope;
}

/** Builds an x1..xn streakable combo sharing one group id. */
export function makeTestGiftCombo(
  count: number,
  overrides: TestGiftOverrides = {},
): DomainEventEnvelope[] {
  const groupId = overrides.groupId ?? `group-combo-${++sequence}`;
  const envelopes: DomainEventEnvelope[] = [];
  for (let step = 1; step <= Math.max(1, count); step += 1) {
    envelopes.push(
      makeTestGiftEnvelope({
        ...overrides,
        groupId,
        repeatCount: step,
        comboCount: step,
        streakable: true,
        repeatEnd: step === Math.max(1, count) && (overrides.repeatEnd ?? true),
      }),
    );
  }
  return envelopes;
}

export function makeTestGapEnvelope(lost = 4): DomainEventEnvelope {
  return { topic: EVENT_GAP_TOPIC, data: { lost, resync: true } };
}

export interface WidgetTestApi {
  emitEnvelope: (envelope: DomainEventEnvelope) => void;
  emitTestFollow: (overrides?: TestFollowOverrides) => void;
  emitTestGift: (overrides?: TestGiftOverrides) => void;
  emitTestGiftCombo: (count: number, overrides?: TestGiftOverrides) => void;
  emitTestChat: (overrides?: TestChatOverrides) => void;
  emitTestShare: (overrides?: TestShareOverrides) => void;
  emitTestSubscribe: (overrides?: TestSubscribeOverrides) => void;
  /** Emulate any trigger by type name (see {@link TEST_EMULATABLE_TYPES}). */
  emitSample: (type: string, overrides?: GenericTestOverrides) => void;
  /**
   * Replay the last live envelope of one type (real gateway data recorded
   * while connected). Returns false when nothing was recorded for the type.
   */
  emitLast: (type: string) => boolean;
  /** Event types with a recorded live envelope available for `emitLast`. */
  lastEventTypes: () => string[];
  /** Current gateway connection status (gateway mode only). */
  connectionStatus: () => string;
}

export function mountWidgetTestHook(api: WidgetTestApi): void {
  if (typeof window === 'undefined') return;
  (window as unknown as Record<string, unknown>)[WIDGET_TEST_HOOK] = api;
}

export function readWidgetTestHook(): WidgetTestApi | null {
  if (typeof window === 'undefined') return null;
  const api = (window as unknown as Record<string, unknown>)[WIDGET_TEST_HOOK];
  if (!api || typeof api !== 'object') return null;
  return api as WidgetTestApi;
}
