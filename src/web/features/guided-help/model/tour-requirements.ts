import type { AutomationActionExampleRequirement, TourRequirementSnapshot } from './types.ts';
export type { TourRequirementSnapshot } from './types.ts';

function actionTypeActive(type: TourRequirementSnapshot['actionTypes'][number], plugins: TourRequirementSnapshot['plugins']): boolean {
  const source = type.source;
  if (source.kind === 'builtin') return true;
  return plugins.some((plugin) => plugin.installed && plugin.enabled && plugin.descriptor.id === source.pluginId);
}

function requirementMatchesType(requirement: AutomationActionExampleRequirement, type: TourRequirementSnapshot['actionTypes'][number]): boolean {
  if (requirement.typeId && type.id !== requirement.typeId) return false;
  if (requirement.tag && type.tag !== requirement.tag) return false;
  return true;
}

/** Whether the action type/tag itself exists and is active. */
export function automationActionTypeAvailable(
  requirement: AutomationActionExampleRequirement,
  snapshot: TourRequirementSnapshot,
): boolean {
  if (!requirement.typeId && !requirement.tag) return false;
  return snapshot.actionTypes.some((type) =>
    requirementMatchesType(requirement, type) && actionTypeActive(type, snapshot.plugins));
}

/** Both halves are required: an available type and an existing user/demo record of that type. */
export function automationActionExampleAvailable(
  requirement: AutomationActionExampleRequirement,
  snapshot: TourRequirementSnapshot,
): boolean {
  if (!requirement.typeId && !requirement.tag) return false;
  return snapshot.actionTypes.some((type) =>
    requirementMatchesType(requirement, type) &&
    actionTypeActive(type, snapshot.plugins) &&
    snapshot.actions.some((action) => action.typeId === type.id));
}


/** Concrete active type behind a requirement, when the host has exactly the type needed to name it for a user. */
export function automationActionExampleType(
  requirement: AutomationActionExampleRequirement,
  snapshot: TourRequirementSnapshot,
): TourRequirementSnapshot['actionTypes'][number] | undefined {
  const matches = snapshot.actionTypes.filter((type) => requirementMatchesType(requirement, type) && actionTypeActive(type, snapshot.plugins));
  return matches.length === 1 ? matches[0] : undefined;
}

/** Semantic operation exposed by Automations for one requirement. */
export function automationActionExampleGuideAction(requirement: AutomationActionExampleRequirement): string {
  if (requirement.typeId) return `automation.openActionType:${requirement.typeId}`;
  if (requirement.tag) return `automation.openActionTag:${requirement.tag}`;
  throw new Error('automation action example requirements need a typeId or tag');
}
