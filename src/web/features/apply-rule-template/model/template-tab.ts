/** Sources of the rule templates dialog: local templates, the Cloud library, a manual file or paste. */
export type RuleTemplateTab = 'templates' | 'cloud' | 'manual';

export function isRuleTemplateTab(value: unknown): value is RuleTemplateTab {
  return value === 'templates' || value === 'cloud' || value === 'manual';
}
