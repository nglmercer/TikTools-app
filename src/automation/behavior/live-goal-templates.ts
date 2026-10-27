import { registerConditionTemplates, type ConditionTemplate } from './condition-templates.ts';
export function liveGoalTemplates(): ConditionTemplate[] {
  return ['global', 'user'].map((scope) => ({
    id: `likes-goal-${scope}`,
    title: { default: `Likes goal (${scope})`, i18key: '' },
    description: { default: 'Runs once when observed likes cross the goal, including packets that skip over it.', i18key: '' },
    triggers: ['tiktok.like'],
    params: [{ key: 'goal', label: { default: 'Likes', i18key: '' }, kind: 'number', default: '15', min: 1, step: 1 }],
    rows: [
      { path: `event.data.aggregation.${scope}.previous`, operator: 'lt', value: { param: 'goal' } },
      { path: `event.data.aggregation.${scope}.total`, operator: 'gte', value: { param: 'goal' } },
    ],
  }));
}
// Opt-in recurring variants. Do not change the two existing one-shot
// template IDs or their previous<goal<=total meaning: imported/saved rules
// must behave exactly as they did before this feature.
export function recurringLikeGoalTemplates(): ConditionTemplate[] {
  return ['global', 'user'].map((scope) => ({
    id: `likes-every-goal-${scope}`,
    title: { default: `Every X likes (${scope})`, i18key: '' },
    description: { default: 'Repeats once per crossed X-like milestone, even when multiple milestones arrive in one batch.', i18key: '' },
    triggers: ['tiktok.like'],
    params: [{ key: 'goal', label: { default: 'Likes per action', i18key: '' }, kind: 'number', default: '150', min: 1, step: 1 }],
    rows: [{ path: `event.data.aggregation.${scope}`, operator: 'crosses-multiple', value: { param: 'goal' } }],
  }));
}
export function registerLiveGoalTemplates(): void {
  registerConditionTemplates([...liveGoalTemplates(), ...recurringLikeGoalTemplates()]);
}
