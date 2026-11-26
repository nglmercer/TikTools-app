import type {
  ActionTypeDefinition,
  BehaviorSnapshot,
  PluginGuideContribution,
  PluginGuideStepContribution,
  PluginGuideTarget,
} from '../../../../automation/behavior/types.ts';
import type { Localized } from '../../../../shared/localized.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import type { TourDefinition, TourRequirementSnapshot, TourStepDefinition } from './types.ts';
import { automationActionTypeAvailable } from './tour-requirements.ts';

export interface DynamicTourDefinition {
  definition: TourDefinition;
  /** Exact synthetic key -> already-resolved text. Unlike host tours, plugin copy is not part of the static locale modules. */
  texts: Readonly<Record<string, string>>;
}

export interface PluginGuideCatalogItem {
  id: string;
  actionType: string;
  mode: 'create' | 'edit' | 'both';
  icon: string;
  title: string;
  description: string;
  steps: readonly { id: string; title: string; description: string }[];
  requirementsMet: boolean;
  dynamic: DynamicTourDefinition;
}

type Snapshot = TourRequirementSnapshot & Pick<BehaviorSnapshot, 'pluginGuides'>;

function object(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function localized(value: unknown): Localized | undefined {
  if (!object(value)) return undefined;
  const fallback = typeof value.default === 'string' ? value.default : '';
  const key = typeof value.i18key === 'string' ? value.i18key : '';
  if (!fallback && !key) return undefined;
  return { default: fallback || key, i18key: key || fallback };
}

function target(value: unknown): PluginGuideTarget | undefined {
  if (!object(value)) return undefined;
  if (value.kind === 'editor' || value.kind === 'settings' || value.kind === 'permissions') return { kind: value.kind };
  if (value.kind === 'field' && typeof value.key === 'string' && value.key) return { kind: 'field', key: value.key };
  return undefined;
}

function step(value: unknown): PluginGuideStepContribution | undefined {
  if (!object(value) || typeof value.id !== 'string' || !/^[a-z0-9_-]{1,64}$/.test(value.id)) return undefined;
  const stepTarget = target(value.target);
  const title = localized(value.title);
  const description = localized(value.description);
  if (!stepTarget || !title || !description) return undefined;
  return {
    id: value.id,
    target: stepTarget,
    title,
    description,
    ...(value.optional === true ? { optional: true } : {}),
  };
}

function actionFieldKeys(type: ActionTypeDefinition | undefined): Set<string> {
  const keys = new Set<string>();
  for (const field of type?.fields ?? []) keys.add(field.key);
  const properties = type?.configSchema?.properties;
  if (object(properties)) for (const key of Object.keys(properties)) keys.add(key);
  return keys;
}

/** Defensive client-side parse of host-stamped guides. The Rust host validates
 * first; this second boundary protects demo hosts, fixtures and older/patched
 * control peers from malformed dynamic tour data. */
export function mergePluginGuides(snapshot: Snapshot): PluginGuideContribution[] {
  const out: PluginGuideContribution[] = [];
  const seen = new Set<string>();
  for (const entry of snapshot.pluginGuides ?? []) {
    if (!object(entry) || typeof entry.id !== 'string' || seen.has(entry.id)) continue;
    if (entry.kind !== 'automation-action' || typeof entry.pluginId !== 'string' || typeof entry.actionType !== 'string') continue;
    if (!entry.id.startsWith(`${entry.pluginId}/`)) continue;
    const mode = entry.mode === 'create' || entry.mode === 'edit' ? entry.mode : 'both';
    const title = localized(entry.title);
    const description = localized(entry.description);
    const steps = Array.isArray(entry.steps) ? entry.steps.map(step).filter((item): item is PluginGuideStepContribution => Boolean(item)) : [];
    const type = snapshot.actionTypes.find((candidate) => candidate.id === entry.actionType) as ActionTypeDefinition | undefined;
    if (!title || !description || !steps.length || !type) continue;
    if (type.source.kind !== 'plugin' || type.source.pluginId !== entry.pluginId) continue;
    const fields = actionFieldKeys(type);
    if (steps.some((item) => item.target.kind === 'field' && !fields.has(item.target.key))) continue;
    if (new Set(steps.map((item) => item.id)).size !== steps.length) continue;
    seen.add(entry.id);
    out.push({
      id: entry.id,
      kind: 'automation-action',
      pluginId: entry.pluginId,
      actionType: entry.actionType,
      mode,
      title,
      description,
      ...(typeof entry.icon === 'string' && entry.icon ? { icon: entry.icon } : {}),
      steps,
      source: { kind: 'plugin', pluginId: entry.pluginId },
    });
  }
  return out;
}

function guideTarget(target: PluginGuideTarget): string {
  if (target.kind === 'editor') return 'automation-editor';
  if (target.kind === 'settings') return 'action-editor-settings';
  if (target.kind === 'permissions') return 'action-editor-permissions';
  return `action-field:${target.key}`;
}

function openAction(guide: PluginGuideContribution): string {
  if (guide.mode === 'edit') return `automation.openActionType:${guide.actionType}`;
  if (guide.mode === 'create') return `automation.createActionType:${guide.actionType}`;
  return `automation.openOrCreateActionType:${guide.actionType}`;
}

/** Convert a validated contribution into the same tour-definition contract
 * the coordinator executes. The host owns routing, preparation and placement;
 * the plugin only supplies closed targets and localized explanatory copy. */
export function pluginGuideTour(guide: PluginGuideContribution): DynamicTourDefinition {
  const texts: Record<string, string> = {};
  const steps: TourStepDefinition[] = guide.steps.map((step) => {
    const titleKey = `${guide.id}.${step.id}.title`;
    const descriptionKey = `${guide.id}.${step.id}.description`;
    texts[titleKey] = localizedText(step.title);
    texts[descriptionKey] = localizedText(step.description);
    const prepareActions = [openAction(guide)];
    if (step.target.kind === 'field') prepareActions.push('automation.revealActionField');
    return {
      id: step.id,
      version: 1,
      target: guideTarget(step.target),
      titleKey,
      descriptionKey,
      requiredCapabilities: [],
      missingTargetPolicy: step.optional ? 'skip' : 'cancel',
      prepareActions,
      popoverSide: 'left',
      popoverAlign: 'start',
    };
  });
  return {
    definition: {
      id: guide.id,
      version: 1,
      route: '/automations',
      scope: 'automations',
      requirements: {
        automationActionType: { typeId: guide.actionType },
        ...(guide.mode === 'edit' ? { automationActionExample: { typeId: guide.actionType } } : {}),
      },
      steps,
    },
    texts,
  };
}

/** Catalog-ready plugin guides for one automation snapshot. Inactive/missing
 * action types fail closed; create/both guides do not require a saved record
 * because their explicit purpose is to guide creation of a draft. */
export function pluginGuideCatalogItems(snapshot: Snapshot | undefined): PluginGuideCatalogItem[] {
  if (!snapshot) return [];
  return mergePluginGuides(snapshot).map((guide) => {
    const dynamic = pluginGuideTour(guide);
    const requirementsMet = automationActionTypeAvailable({ typeId: guide.actionType }, snapshot) &&
      (guide.mode !== 'edit' || snapshot.actions.some((action) => action.typeId === guide.actionType));
    return {
      id: guide.id,
      actionType: guide.actionType,
      mode: guide.mode,
      icon: guide.icon ?? 'puzzle',
      title: localizedText(guide.title),
      description: localizedText(guide.description),
      steps: guide.steps.map((step) => ({ id: step.id, title: localizedText(step.title), description: localizedText(step.description) })),
      requirementsMet,
      dynamic,
    };
  });
}
