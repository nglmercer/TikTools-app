import { createActionId, createEventId } from './schema.ts';
import type { LiveAction, LiveEvent } from './types.ts';

/**
 * Web mirror of `tiktools-core::services::automation::moderation`: the
 * TextIntel processor stays side-effect free and only attaches
 * `moderation.blocked` evidence under its provider namespace. Deducting
 * points is automation work — one `core.points` action plus the behavior
 * event wiring `tiktok.chat` through the verdict filter to it.
 *
 * The constants below must stay identical to the Rust builders; the records
 * additionally carry the web shape (client-minted ids, schema versions,
 * event defaults) that the behavior save path expects.
 */

/** Filter path addressing the TextIntel moderation verdict on enriched chat. */
export const MODERATION_BLOCKED_PATH =
  'event.intel.providers.textintel.comment.moderation.blocked';
/** Executable action type carrying the penalty. Never a second points system. */
export const MODERATION_POINTS_ACTION_TYPE = 'core.points';
/** Viewer template resolved per event by the automation engine. */
export const MODERATION_VIEWER_TEMPLATE = '{{ event.user.uniqueId }}';

export const MODERATION_ACTION_NAME = 'Moderation penalty';
export const MODERATION_EVENT_NAME = 'TextIntel moderation penalty';

export type PenaltyPointsError = 'not-finite' | 'zero';

/**
 * Mirrors `validate_penalty_points`: finite and non-zero, so the stored
 * `core.points` action always executes. Negative values deduct; positive
 * values award. Returns the error code for i18n lookup, or undefined.
 */
export function penaltyPointsError(points: number): PenaltyPointsError | undefined {
  if (!Number.isFinite(points)) return 'not-finite';
  if (points === 0) return 'zero';
  return undefined;
}

/** Builds the `core.points` action record for a moderation penalty. */
export function moderationPenaltyAction(points: number): LiveAction {
  return {
    schemaVersion: 2,
    id: createActionId(),
    name: MODERATION_ACTION_NAME,
    typeId: MODERATION_POINTS_ACTION_TYPE,
    enabled: true,
    config: {
      uniqueId: MODERATION_VIEWER_TEMPLATE,
      delta: points,
    },
  };
}

/** Builds the behavior event wiring `tiktok.chat` through the moderation
 * filter to one stored points action. */
export function moderationPenaltyEvent(actionId: string): LiveEvent {
  return {
    schemaVersion: 1,
    id: createEventId(),
    name: MODERATION_EVENT_NAME,
    enabled: true,
    trigger: 'tiktok.chat',
    filters: [{ path: MODERATION_BLOCKED_PATH, operator: 'is-true', value: '' }],
    cooldownMs: 0,
    cooldownScope: 'user',
    actionIds: [actionId],
    runMode: 'all',
  };
}
