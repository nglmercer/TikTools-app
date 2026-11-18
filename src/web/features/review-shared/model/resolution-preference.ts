import { computed, ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';

export type ResolutionMode = 'automatic' | 'manual';

export const RESOLUTION_MODE_KEY = 'app.resolution.mode';
export const DEFAULT_RESOLUTION_MODE: ResolutionMode = 'automatic';

/** Conflicts and unconfirmed outcomes are resolved automatically by default:
 * the current record is read and adopted as the new base without replaying the
 * original action. Manual review is an opt-in advanced mode kept for later. */
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
    resolutionMode.value = stored.state[RESOLUTION_MODE_KEY] === 'manual' ? 'manual' : 'automatic';
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
