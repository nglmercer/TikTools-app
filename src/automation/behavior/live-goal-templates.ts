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
export function registerLiveGoalTemplates(): void { registerConditionTemplates(liveGoalTemplates()); }
