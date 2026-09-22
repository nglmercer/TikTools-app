/**
 * Development-only fake event factories plus the page test hook. Production
 * widget behavior never depends on this API; the hook exists so tests and
 * manual previews can inject envelopes without a TikTok connection.
 */

import { EVENT_GAP_TOPIC, LIVE_EVENT_TOPIC } from './config.ts';
import type {
  AutomationUser,
  DomainEventEnvelope,
  FollowAutomationEvent,
  GiftAutomationEvent,
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

export function toLiveEnvelope(
  event: FollowAutomationEvent | GiftAutomationEvent,
): DomainEventEnvelope {
  return { topic: LIVE_EVENT_TOPIC, data: { eventType: event.type, event } };
}

export function makeTestFollowEnvelope(overrides?: TestFollowOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestFollowEvent(overrides));
}

export function makeTestGiftEnvelope(overrides?: TestGiftOverrides): DomainEventEnvelope {
  return toLiveEnvelope(makeTestGiftEvent(overrides));
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
