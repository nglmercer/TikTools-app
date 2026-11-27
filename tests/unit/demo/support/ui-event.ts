import type { UiEvent } from '../../../../src/shared/messages.ts';
import { isRecord, propertyOf } from '../../support/json.ts';

const KINDS: readonly UiEvent['kind'][] = ['chat', 'gift', 'like', 'member', 'social'];

const isKind = (value: unknown): value is UiEvent['kind'] => KINDS.some((kind) => kind === value);

const optionalNumber = (raw: unknown, key: string): number | undefined => {
  const value = propertyOf(raw, key);
  return typeof value === 'number' ? value : undefined;
};

const requiredString = (raw: unknown, key: string): string => {
  const value = propertyOf(raw, key);
  if (typeof value !== 'string') throw new Error(`Expected feed event "${key}" to be a string`);
  return value;
};

function toGiftDetails(raw: unknown): UiEvent['giftDetails'] {
  if (!isRecord(raw)) return undefined;
  const imageUrl = propertyOf(raw, 'imageUrl');
  const count = optionalNumber(raw, 'count');
  const diamonds = optionalNumber(raw, 'diamonds');
  if (count === undefined || diamonds === undefined) throw new Error('Expected gift details to carry numeric count and diamonds');
  return { name: requiredString(raw, 'name'), count, diamonds, ...(typeof imageUrl === 'string' ? { imageUrl } : {}) };
}

/** Reads a demo feed-journal row into the UI event contract, checking the fields the tests rely on at runtime. */
export function toUiEvent(raw: unknown): UiEvent {
  const kind = propertyOf(raw, 'kind');
  if (!isKind(kind)) throw new Error(`Unknown feed event kind ${String(kind)}`);
  return {
    kind,
    author: requiredString(raw, 'author'),
    text: requiredString(raw, 'text'),
    points: optionalNumber(raw, 'points'),
    level: optionalNumber(raw, 'level'),
    giftDetails: toGiftDetails(propertyOf(raw, 'giftDetails')),
  };
}
