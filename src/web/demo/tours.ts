import type { TourId } from '@/features/guided-help/model/types.ts';
import { scenarios, type ScenarioId } from './scenarios.ts';

/** The signed-out demo has no cloud account, so the account and backup tour is real-app only. */
export type DemoTourId = Exclude<TourId, 'backup'>;
export const isDemoTour = (id: string): id is DemoTourId => Object.hasOwn(demoTourScenarios, id);

export const demoTourScenarios = {
  connection: 'connection', automation: 'automation', plugin: 'plugins', overlay: 'overlays',
  points: 'points', changes: 'cloud', diagnostics: 'diagnostics', feed: 'feed',
} as const satisfies Record<DemoTourId, ScenarioId>;

export function scenarioTour(scenario: ScenarioId): DemoTourId {
  return (Object.keys(demoTourScenarios) as DemoTourId[]).find((id) => demoTourScenarios[id] === scenario) ?? 'connection';
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
