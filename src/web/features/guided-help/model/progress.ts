import type { TourProgress } from './types.ts';

const key = (id: string, version: number) => `tiktools.guide.tour.${id}.v${version}`;

/** Storage is optional and untrusted. A prior tour version cannot complete a new tour. */
export function readTourProgress(storage: Storage | undefined, id: string, version: number, stepIds: readonly string[]): TourProgress | undefined {
  try {
    const raw: unknown = JSON.parse(storage?.getItem(key(id, version)) ?? 'null');
    if (!raw || typeof raw !== 'object') return;
    const value = raw as Record<string, unknown>;
    if (value.version !== version || typeof value.stepId !== 'string' || !stepIds.includes(value.stepId) ||
        !['in-progress', 'dismissed', 'completed'].includes(String(value.status))) return;
    return { version, stepId: value.stepId, status: value.status as TourProgress['status'] };
  } catch { /* Restricted/quota-limited storage must not prevent help. */ }
  return;
}

export function writeTourProgress(storage: Storage | undefined, id: string, progress: TourProgress): void {
  try { storage?.setItem(key(id, progress.version), JSON.stringify(progress)); }
  catch { /* Progress is advisory, never an authority or a business journal. */ }
}
