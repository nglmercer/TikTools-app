import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { updaterApi } from '../api/updater-api.ts';
import type { UpdateChannel, UpdatePreferences, UpdateStatus } from '../api/updater-types.ts';

const KEY = 'app.update.preferences';
const INTERVAL_SECONDS = 12 * 60 * 60;

export const useAppUpdaterStore = defineStore('app-updater', () => {
  const client = useControlClient();
  const status = shallowRef<UpdateStatus | null>(null);
  const preferences = ref<UpdatePreferences>({ channel: 'stable', automaticChecks: true, notifications: true, lastChecked: 0, lastAttempt: 0 });
  const error = ref('');
  let initialized = false;
  let saveQueue = Promise.resolve();

  function save(): Promise<void> {
    const value = JSON.stringify(preferences.value);
    saveQueue = saveQueue.then(async () => {
      await client.call('app.state.set', { key: KEY, value });
    }).catch((failure: unknown) => { error.value = errorMessage(failure); });
    return saveQueue;
  }

  async function initialize(): Promise<void> {
    if (initialized) return;
    initialized = true;
    client.onTopic<{ state: UpdateStatus }>('app.update.stateChanged', ({ state }) => {
      status.value = { ...state, checkSupported: status.value?.checkSupported ?? false };
      if (state.lastChecked && state.lastChecked > preferences.value.lastChecked) {
        preferences.value.lastChecked = state.lastChecked;
        void save();
      }
    });
    try {
      const stored = await client.call<{ state: Record<string, string> }>('app.state.get', { keys: [KEY] });
      const parsed = JSON.parse(stored.state[KEY] || '{}') as Partial<UpdatePreferences>;
      preferences.value = {
        channel: parsed.channel === 'beta' ? 'beta' : 'stable',
        automaticChecks: parsed.automaticChecks !== false,
        notifications: parsed.notifications !== false,
        lastChecked: typeof parsed.lastChecked === 'number' && Number.isFinite(parsed.lastChecked) ? parsed.lastChecked : 0,
        lastAttempt: typeof parsed.lastAttempt === 'number' && Number.isFinite(parsed.lastAttempt) ? parsed.lastAttempt : 0,
      };
      status.value = await updaterApi.status(client);
      if (status.value.checkSupported && preferences.value.automaticChecks && Date.now() / 1000 - preferences.value.lastAttempt >= INTERVAL_SECONDS) await check();
    } catch (failure) { error.value = errorMessage(failure); }
  }

  async function check(): Promise<void> {
    error.value = '';
    preferences.value.lastAttempt = Math.floor(Date.now() / 1000);
    await save();
    try { await updaterApi.check(client, preferences.value.channel); }
    catch (failure) { error.value = errorMessage(failure); }
  }

  async function download(): Promise<void> {
    error.value = '';
    try { await updaterApi.download(client); }
    catch (failure) { error.value = errorMessage(failure); }
  }

  async function setChannel(channel: UpdateChannel): Promise<void> { preferences.value.channel = channel; await save(); }
  async function setPreference(key: 'automaticChecks' | 'notifications', value: boolean): Promise<void> { preferences.value[key] = value; await save(); }
  return { status, preferences, error, initialize, check, download, setChannel, setPreference };
});
