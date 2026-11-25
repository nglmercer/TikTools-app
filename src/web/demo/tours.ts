import { tourRegistry } from '@/features/guided-help/model/registry.ts';
import type { TourId } from '@/features/guided-help/model/types.ts';
import { scenarios, type ScenarioId } from './scenarios.ts';

/** Cloud-account and experimental ranking tours are real-app only; the signed-out demo cannot honestly prepare those states. */
export type DemoTourId = Exclude<TourId, 'backup' | 'cloudOverlays' | 'rankings' | 'backupRestore'>;
export const isDemoTour = (id: string): id is DemoTourId => Object.hasOwn(demoTourScenarios, id);

const scenarioOverrides: Partial<Record<DemoTourId, ScenarioId>> = {
  automationVariables: 'cloud',
  changes: 'cloud',
};

/** Tour scopes and demo scenarios name the same sections differently (automations/automation). */
const scopeScenarios: Record<string, ScenarioId> = {
  connections: 'connection',
  automations: 'automation',
  account: 'cloud',
  feed: 'feed',
  points: 'points',
  analytics: 'analytics',
  overlays: 'overlays',
  plugins: 'plugins',
  settings: 'settings',
  diagnostics: 'diagnostics',
};

/** Scenarios derive from the tour scope; only exceptions need an override. */
export const demoTourScenarios = Object.fromEntries(
  Object.values(tourRegistry)
    .filter((tour) => !['backup', 'cloudOverlays', 'rankings', 'backupRestore'].includes(tour.id))
    .map((tour) => [tour.id, scenarioOverrides[tour.id as DemoTourId] ?? scopeScenarios[tour.scope] ?? tour.scope]),
) as Record<DemoTourId, ScenarioId>;

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
