/**
 * Last-live-envelope memory for widget emulation. The runtime records each
 * real gateway envelope per event type so the test hook can replay actual
 * data (`emitLast`) instead of synthetic samples.
 *
 * In-memory only, on purpose: replaying yesterday's viewer from disk would
 * be stale and surprising, and persisting chat text invites PII retention
 * questions. A reload starts with an empty cache and honest `false` from
 * `replay`.
 */

import { parseLiveEvent } from './event-parser.ts';
import type { AutomationEventLike, DomainEventEnvelope } from './event-types.ts';

function cloneEnvelope(envelope: DomainEventEnvelope): DomainEventEnvelope {
  return structuredClone(envelope) as DomainEventEnvelope;
}

export class LastEventCache {
  private readonly envelopes = new Map<string, DomainEventEnvelope>();

  /** Event types with a recorded envelope, in first-seen order. */
  get types(): string[] {
    return [...this.envelopes.keys()];
  }

  get size(): number {
    return this.envelopes.size;
  }

  /**
   * Records one gateway frame when it carries a live event. Non-live topics
   * and malformed frames are ignored (never throw).
   */
  record(value: unknown): void {
    if (!isEnvelopeLike(value)) return;
    const payload = parseLiveEvent(value);
    if (!payload) return;
    this.envelopes.set(payload.eventType, cloneEnvelope(value));
  }

  /** True when a live envelope of this type was recorded. */
  has(eventType: string): boolean {
    return this.envelopes.has(eventType);
  }

  /**
   * Fresh replayable copy of the last live envelope of one type: same
   * payload, new id and timestamp so re-emulation never trips duplicate
   * protection. Returns null when nothing was recorded.
   */
  replay(eventType: string): DomainEventEnvelope | null {
    const stored = this.envelopes.get(eventType);
    if (!stored) return null;
    const envelope = cloneEnvelope(stored);
    const event = (envelope.data as { event: AutomationEventLike }).event;
    event.id = `${event.id}-replay-${Date.now()}`;
    event.timestamp = Date.now();
    return envelope;
  }

  clear(): void {
    this.envelopes.clear();
  }
}

function isEnvelopeLike(value: unknown): value is DomainEventEnvelope {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    typeof (value as Record<string, unknown>)['topic'] === 'string' &&
    'data' in value
  );
}
