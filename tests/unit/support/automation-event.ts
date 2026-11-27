import type { AutomationEvent } from '../../../src/automation/types.ts';
import { isRecord } from './json.ts';

/** A complete automation event with defaults for the envelope fields; override only what a test cares about. */
export function automationEvent(overrides: Partial<AutomationEvent> & Pick<AutomationEvent, 'type'>): AutomationEvent {
  return { id: 'event-1', timestamp: 0, data: {}, ...overrides };
}

type CommentIntel = NonNullable<NonNullable<AutomationEvent['intel']>['comment']>;
type Composition = NonNullable<CommentIntel['composition']>;
type Spam = NonNullable<CommentIntel['spam']>;

/** A complete `intel.comment` block where every analysis is absent unless the test supplies it. */
interface CommentIntelParts {
  composition?: Partial<Composition>;
  spam?: Partial<Spam>;
}

export function commentIntel(parts: CommentIntelParts = {}): NonNullable<AutomationEvent['intel']> {
  const composition: Composition = {
    allCaps: null,
    digits: null,
    elongated: null,
    emojiCount: null,
    emojiOnly: null,
    emojiRatio: null,
    letters: null,
    mentions: null,
    repetitionScore: null,
    urls: null,
    ...parts.composition,
  };
  const spam: Spam = { calibrated: null, detected: null, model: null, reasons: null, score: null, ...parts.spam };
  return {
    comment: {
      casefolded: null,
      composition,
      language: null,
      nfc: null,
      nfkc: null,
      normalized: null,
      obfuscation: null,
      rebus: null,
      spam,
      truncated: false,
      tts: null,
      unicode: null,
    },
    processing: null,
    providers: {},
    user: null,
  };
}

/** The `data` payload of an event as an object; throws when the event carries a non-object payload. */
export function dataOf(event: AutomationEvent): Record<string, unknown> {
  const { data } = event;
  if (!isRecord(data)) throw new Error(`Expected event "${event.type}" to carry an object payload`);
  return data;
}
