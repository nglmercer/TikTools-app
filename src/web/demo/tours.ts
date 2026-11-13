import type { TourId } from '@/features/guided-help/model/types.ts';
import { scenarios, type ScenarioId } from './scenarios.ts';

export const demoTourScenarios = {
  connection: 'connection', automation: 'automation', plugin: 'plugins', overlay: 'overlays',
  points: 'points', changes: 'cloud', diagnostics: 'diagnostics',
} as const satisfies Record<TourId, ScenarioId>;

export function scenarioTour(scenario: ScenarioId): TourId {
  return (Object.keys(demoTourScenarios) as TourId[]).find((id) => demoTourScenarios[id] === scenario) ?? 'connection';
}

/** A fresh scenario seeds all required fixtures; no tutorial imports a production account. */
export function demoTourUrl(current: string, id: TourId): string {
  const url = new URL(current);
  const scenario = demoTourScenarios[id];
  url.searchParams.set('scenario', scenario);
  url.searchParams.set('tour', id);
  url.searchParams.delete('route');
  url.hash = scenarios[scenario].route;
  return url.href;
}
