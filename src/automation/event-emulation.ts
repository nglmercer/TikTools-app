import { sampleEventForType } from './event-registry.ts';
import type { AutomationEvent, JsonObject } from './types.ts';

/**
 * Shared emulation policy for every "run once with an event" surface:
 * processor previews, template autocomplete, and (mirrored by the host in
 * `automation_runtime/dispatch.rs`) the behavior Test button.
 *
 * Rule: replay the last live envelope of the trigger when one matches (real
 * user, real gift, real counts), else fall back to the registry sample.
 * Replays are cloned with a fresh timestamp so time-based logic behaves.
 */

export type EmulatedEventSource = 'live' | 'sample';

export interface EmulatedEvent {
  event: AutomationEvent;
  source: EmulatedEventSource;
}

/** The last live event when it matches the trigger being emulated. */
export function matchingLastEvent(
  eventType: string | undefined,
  lastEvent?: AutomationEvent | null,
): AutomationEvent | undefined {
  if (!lastEvent) return undefined;
  if (eventType && lastEvent.type !== eventType) return undefined;
  return lastEvent;
}

/**
 * Emulate one trigger against the best available data. A matching live
 * envelope wins over the sample; mismatched or absent live data degrades to
 * the registry sample instead of failing.
 */
export function emulateEventForType(
  eventType: string,
  lastEvent?: AutomationEvent | null,
): EmulatedEvent {
  const matched = matchingLastEvent(eventType, lastEvent);
  if (matched) {
    const event = structuredClone(matched) as AutomationEvent;
    event.timestamp = Date.now();
    return { event, source: 'live' };
  }
  return { event: sampleEventForType(eventType), source: 'sample' };
}

/**
 * Fresh identity for a replayed envelope, so re-emulation never trips
 * duplicate protection keyed by event id. The payload is untouched.
 */
export function refreshEmulatedEvent<T extends AutomationEvent>(event: T, id?: string): T {
  const clone = structuredClone(event) as T;
  clone.timestamp = Date.now();
  if (id !== undefined) clone.id = id;
  return clone;
}

export interface GiftComboOverrides {
  groupId?: string;
  giftId?: string;
  giftName?: string;
  diamondCount?: number;
  /** Last step sets `repeatEnd` unless explicitly overridden. */
  repeatEnd?: boolean;
  user?: AutomationEvent['user'];
}

let comboSequence = 0;

/**
 * Registry-based gift streak (x1..xn) sharing one group id. Built from the
 * canonical gift sample so streak previews carry the same shape as live
 * streakable gifts.
 */
export function makeGiftComboEvents(
  count: number,
  overrides: GiftComboOverrides = {},
): AutomationEvent[] {
  comboSequence += 1;
  const steps = Math.max(1, count);
  const groupId = overrides.groupId ?? `sample-combo-${comboSequence}`;
  const events: AutomationEvent[] = [];
  for (let step = 1; step <= steps; step += 1) {
    const event = sampleEventForType('tiktok.gift');
    const data = event.data as JsonObject;
    data['groupId'] = groupId;
    data['repeatCount'] = step;
    data['comboCount'] = step;
    data['streakable'] = true;
    data['repeatEnd'] = step === steps ? (overrides.repeatEnd ?? true) : false;
    if (overrides.giftId !== undefined) data['giftId'] = overrides.giftId;
    if (overrides.giftName !== undefined) data['giftName'] = overrides.giftName;
    if (overrides.diamondCount !== undefined) data['diamondCount'] = overrides.diamondCount;
    if (overrides.user !== undefined) event.user = overrides.user;
    event.id = `sample-event-combo-${comboSequence}-${step}`;
    events.push(event);
  }
  return events;
}
