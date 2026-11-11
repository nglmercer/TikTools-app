import type { DomainEventEnvelope } from './event-types.ts';
import { displayNameFor, handleFor } from './event-types.ts';
import { isChatEvent, isGiftEvent, parseLiveEvent } from './event-parser.ts';

/** Live values an overlay element may show. Only these fields exist; the Cloud allows no others. */
export type LiveOverlayField = 'viewer.name' | 'viewer.handle' | 'gift.name' | 'gift.count' | 'gift.diamonds' | 'chat.message' | 'points.total';
export type LiveFieldValues = Partial<Record<LiveOverlayField, string>>;

/**
 * Extracts the live field values one gateway envelope carries. Envelopes without a
 * field simply leave it unchanged; nothing is invented.
 */
export function liveFieldsFrom(envelope: DomainEventEnvelope): LiveFieldValues {
  const payload = parseLiveEvent(envelope);
  if (!payload) return {};
  const event = payload.event;
  const values: LiveFieldValues = {};
  if (event.user) {
    values['viewer.name'] = displayNameFor(event.user);
    values['viewer.handle'] = handleFor(event.user);
  }
  if (isGiftEvent(event)) {
    values['gift.name'] = event.data.giftName;
    const count = Math.max(Math.floor(event.data.repeatCount), Math.floor(event.data.comboCount), 1);
    values['gift.count'] = String(count);
    values['gift.diamonds'] = String(Math.max(Math.floor(event.data.diamondCount), 0));
  }
  if (isChatEvent(event)) values['chat.message'] = event.data.comment;
  return values;
}
