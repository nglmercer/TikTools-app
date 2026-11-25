import { tourRegistry } from '@/features/guided-help/model/registry.ts';
import type { TourId } from '@/features/guided-help/model/types.ts';
import { scenarios, type ScenarioId } from './scenarios.ts';

/** Cloud-account and experimental ranking tours are real-app only; the signed-out demo cannot honestly prepare those states. */
export type DemoTourId = Exclude<TourId, 'backup' | 'cloudOverlays' | 'rankings' | 'backupRestore'>;
export const isDemoTour = (id: string): id is DemoTourId => Object.hasOwn(demoTourScenarios, id);

export const demoTourScenarios = {
  connection: 'connection', automationEvents: 'automation', automationActions: 'automation', automationVariables: 'cloud',
  plugin: 'plugins', overlay: 'overlays',
  points: 'points', changes: 'cloud', diagnostics: 'diagnostics', feed: 'feed',
  analytics: 'analytics', settings: 'settings', overlayObs: 'overlays',
  pluginSetup: 'plugins', connectionRecovery: 'connection', feedInspector: 'feed',
  automationEventChat: 'automation', automationEventGift: 'automation', automationEventSocial: 'automation',
  automationEventRoom: 'automation', automationActionMedia: 'automation', automationActionTts: 'automation',
  automationActionHttp: 'automation', automationActionVars: 'automation', automationFilters: 'automation',
  automationCooldowns: 'automation', automationTemplates: 'automation',
} as const satisfies Record<DemoTourId, ScenarioId>;

/** Tours offered by the demo catalog, in catalog order. */
export const demoTutorialIds = Object.keys(demoTourScenarios) as DemoTourId[];

export function scenarioTour(scenario: ScenarioId): DemoTourId {
  return demoTutorialIds.find((id) => demoTourScenarios[id] === scenario) ?? 'connection';
}

/** Recommend the tour for the section the reader is actually looking at. */
export function tourForRoute(fullPath: string, scenario: ScenarioId): DemoTourId {
  const [pathAndQuery] = fullPath.split('#');
  const [path = '', query = ''] = (pathAndQuery ?? '').split('?');
  if (path === '/automations') return query.includes('view=variables') ? 'automationVariables' : 'automationEvents';
  return demoTutorialIds.find((id) => {
    const tourPath = tourRegistry[id].route.split('?')[0] ?? '';
    return path === tourPath || path.startsWith(`${tourPath}/`);
  }) ?? scenarioTour(scenario);
}

/** A fresh scenario seeds all required fixtures; no tutorial imports a production account. */
export function demoTourUrl(current: string, id: DemoTourId): string {
  const url = new URL(current);
  const scenario = demoTourScenarios[id];
  url.searchParams.set('scenario', scenario);
  url.searchParams.set('tour', id);
  url.searchParams.delete('route');
  url.hash = scenarios[scenario].route;
  return url.href;
}
