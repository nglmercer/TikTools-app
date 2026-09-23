import {
  registerConditionTemplates,
  type ConditionTemplate,
} from './condition-templates.ts';

/**
 * Moderation condition templates for chat: three strictness levels over the
 * TextIntel enrichment. Each level exposes its repetition threshold as a
 * config param (the modal default), so the shipped numbers are starting
 * points, not magic constants; High additionally requires the spam flag.
 *
 * Higher level means stricter: the threshold drops, so more messages pass
 * the filter and reach the action.
 */
export function moderationConditionTemplates(): ConditionTemplate[] {
  const thresholdParam = (fallback: string): ConditionTemplate['params'] => [
    {
      key: 'threshold',
      label: { default: 'Repetition threshold', i18key: 'condition.moderationThreshold' },
      hint: { default: 'Scores at or above this value pass.', i18key: 'condition.moderationThresholdHint' },
      kind: 'number',
      default: fallback,
      min: 0,
      max: 1,
      step: 0.05,
    },
  ];
  const repetitionRow = { path: 'event.intel.comment.composition.repetitionScore', operator: 'gte' as const, value: { param: 'threshold' } };
  return [
    {
      id: 'moderation-low',
      title: { default: 'Moderation: Low', i18key: 'condition.moderationLow' },
      description: { default: 'Only blatant repetition trips this level.', i18key: 'condition.moderationLowDesc' },
      triggers: ['tiktok.chat'],
      params: thresholdParam('0.9'),
      rows: [{ ...repetitionRow }],
    },
    {
      id: 'moderation-medium',
      title: { default: 'Moderation: Medium', i18key: 'condition.moderationMedium' },
      description: { default: 'Balanced: catches likely spam.', i18key: 'condition.moderationMediumDesc' },
      triggers: ['tiktok.chat'],
      params: thresholdParam('0.7'),
      rows: [{ ...repetitionRow }],
    },
    {
      id: 'moderation-high',
      title: { default: 'Moderation: High', i18key: 'condition.moderationHigh' },
      description: { default: 'Strict: low threshold plus flagged spam.', i18key: 'condition.moderationHighDesc' },
      triggers: ['tiktok.chat'],
      params: thresholdParam('0.5'),
      rows: [
        { ...repetitionRow },
        { path: 'event.intel.comment.spam.detected', operator: 'is-true', value: '' },
      ],
    },
  ];
}

/** Registers the moderation templates. Idempotent: re-registering replaces by id. */
export function registerModerationTemplates(): void {
  registerConditionTemplates(moderationConditionTemplates());
}
