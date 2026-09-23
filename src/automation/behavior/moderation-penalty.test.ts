import { describe, expect, test } from 'bun:test';

import {
  MODERATION_ACTION_NAME,
  MODERATION_BLOCKED_PATH,
  MODERATION_EVENT_NAME,
  MODERATION_POINTS_ACTION_TYPE,
  MODERATION_VIEWER_TEMPLATE,
  moderationPenaltyAction,
  moderationPenaltyEvent,
  penaltyPointsError,
} from './moderation-penalty.ts';

describe('moderation penalty builders', () => {
  test('action record mirrors the core builder shape', () => {
    const action = moderationPenaltyAction(-10);
    expect(action.schemaVersion).toBe(2);
    expect(action.id.startsWith('act-')).toBe(true);
    expect(action.name).toBe(MODERATION_ACTION_NAME);
    expect(action.typeId).toBe(MODERATION_POINTS_ACTION_TYPE);
    expect(action.typeId).toBe('core.points');
    expect(action.enabled).toBe(true);
    expect(action.config.uniqueId).toBe(MODERATION_VIEWER_TEMPLATE);
    expect(action.config.uniqueId).toBe('{{ event.user.uniqueId }}');
    // The executor accepts numbers and numeric strings; builders emit a
    // number, exactly like the Rust record builder.
    expect(action.config.delta).toBe(-10);
  });

  test('each action gets a fresh id', () => {
    expect(moderationPenaltyAction(-10).id).not.toBe(moderationPenaltyAction(-10).id);
  });

  test('event record wires chat through the moderation filter to the action', () => {
    const event = moderationPenaltyEvent('act-1');
    expect(event.schemaVersion).toBe(1);
    expect(event.id.startsWith('evt-')).toBe(true);
    expect(event.name).toBe(MODERATION_EVENT_NAME);
    expect(event.enabled).toBe(true);
    expect(event.trigger).toBe('tiktok.chat');
    expect(event.filters).toEqual([
      { path: MODERATION_BLOCKED_PATH, operator: 'is-true', value: '' },
    ]);
    expect(event.actionIds).toEqual(['act-1']);
    expect(event.runMode).toBe('all');
    expect(event.cooldownMs).toBe(0);
  });

  test('filter path matches the provider-namespaced verdict', () => {
    expect(MODERATION_BLOCKED_PATH).toBe(
      'event.intel.providers.textintel.comment.moderation.blocked',
    );
  });
});

describe('penalty validation', () => {
  test('accepts finite non-zero amounts of either sign', () => {
    expect(penaltyPointsError(-10)).toBeUndefined();
    expect(penaltyPointsError(5)).toBeUndefined();
    expect(penaltyPointsError(-0.5)).toBeUndefined();
  });

  test('rejects zero like the core validator', () => {
    expect(penaltyPointsError(0)).toBe('zero');
    expect(penaltyPointsError(-0)).toBe('zero');
  });

  test('rejects non-finite input like the core validator', () => {
    expect(penaltyPointsError(Number.NaN)).toBe('not-finite');
    expect(penaltyPointsError(Number.POSITIVE_INFINITY)).toBe('not-finite');
    expect(penaltyPointsError(Number.NEGATIVE_INFINITY)).toBe('not-finite');
  });
});
