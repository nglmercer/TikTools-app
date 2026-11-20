import { computed, ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';

export type ResolutionMode = 'automatic' | 'manual';

export const RESOLUTION_MODE_KEY = 'app.resolution.mode';
export const DEFAULT_RESOLUTION_MODE: ResolutionMode = 'manual';

/** Conflicts and unconfirmed outcomes go to a manual review queue by default;
 * nothing is resolved automatically unless the user explicitly opts in to the
 * advanced automatic mode. */
export const resolutionMode = ref<ResolutionMode>(DEFAULT_RESOLUTION_MODE);
export const manualReview = computed(() => resolutionMode.value === 'manual');

let bound: ControlClient | null = null;
let loaded = false;

export async function loadResolutionPreference(client: ControlClient): Promise<void> {
  bound = client;
  if (loaded) return;
  loaded = true;
  try {
    const stored = await client.call<{ state: Record<string, string> }>('app.state.get', { keys: [RESOLUTION_MODE_KEY] });
    resolutionMode.value = stored.state[RESOLUTION_MODE_KEY] === 'automatic' ? 'automatic' : DEFAULT_RESOLUTION_MODE;
  } catch {
    resolutionMode.value = DEFAULT_RESOLUTION_MODE;
  }
}

export async function setResolutionMode(client: ControlClient, mode: ResolutionMode): Promise<void> {
  await client.call('app.state.set', { key: RESOLUTION_MODE_KEY, value: mode });
  bound = client;
  resolutionMode.value = mode;
}

/** Used by tests and by flows without a control client. */
export function resetResolutionPreference(mode: ResolutionMode = DEFAULT_RESOLUTION_MODE): void {
  resolutionMode.value = mode;
  bound = null;
  loaded = false;
}

export function hasResolutionClient(): boolean {
  return bound !== null;
}
