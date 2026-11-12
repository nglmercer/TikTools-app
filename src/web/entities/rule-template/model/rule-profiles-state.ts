import { ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import type { AppliedRuleTemplate } from '../lib/rule-templates.ts';
import { DEFAULT_PROFILE_ID, resolvedPacks, slugProfileId, type ProfilePack, type RuleProfile, type ProfileUniverse } from '../lib/rule-profiles.ts';
import { PROFILE_PACKS_KEY, ACTIVE_PROFILE_KEY, parseStoredProfilePacks } from './profile-pack-storage.ts';

// Keep the existing entry point for profile document consumers and stores.
export { PROFILE_VERSION, MAX_PROFILE_ENTRIES, DEFAULT_PROFILE_ID, MAX_PROFILE_TEXT_BYTES,
  parseRuleProfile, parseRuleProfileImport, mergedEntryParams, instantiateProfile, profileParamSchema,
  resolvedPacks, slugProfileId,
} from '../lib/rule-profiles.ts';
export type { RuleProfileEntry, RuleProfile, ProfilePack, ProfileUniverse } from '../lib/rule-profiles.ts';
export { buildProfileDoc, downloadTextFile } from '../lib/profile-document.ts';
export type { ExportedProfileDoc } from '../lib/profile-document.ts';

export function useRuleProfiles(control: ControlClient, persistence: () => boolean | null = () => null) {
  const packs = ref<ProfilePack[]>([]);
  const activeId = ref<string>(DEFAULT_PROFILE_ID);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const notice = ref<string | null>(null);

  const loadProfiles = async (): Promise<void> => {
    loading.value = true;
    error.value = null;
    try {
      const result = await control.call<{ state: Record<string, string> }>('app.state.get', {
        keys: [PROFILE_PACKS_KEY, ACTIVE_PROFILE_KEY],
      });
      const parsed = parseStoredProfilePacks(result.state[PROFILE_PACKS_KEY]);
      packs.value = parsed.packs;
      if (parsed.errors.length > 0) error.value = parsed.errors.join('; ');
      const active = (result.state[ACTIVE_PROFILE_KEY] ?? '').trim();
      activeId.value =
        active !== '' && (active === DEFAULT_PROFILE_ID || parsed.packs.some((pack) => pack.id === active))
          ? active
          : DEFAULT_PROFILE_ID;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  };

  const persist = async (next: ProfilePack[], active: string): Promise<void> => {
    if (persistence() !== false) throw new ControlCallError('capability_unavailable', 'Persistent profile changes require the native atomic library.');
    await Promise.all([
      control.call('app.state.set', { key: PROFILE_PACKS_KEY, value: JSON.stringify(next) }),
      control.call('app.state.set', { key: ACTIVE_PROFILE_KEY, value: active }),
    ]);
    packs.value = next;
    activeId.value = active;
  };

  const registerPack = async (profile: RuleProfile, applied: AppliedRuleTemplate[]): Promise<void> => {
    if (profile.id === DEFAULT_PROFILE_ID) throw new Error('profile id "default" is reserved');
    error.value = null;
    const pack: ProfilePack = {
      id: profile.id,
      name: profile.name,
      description: profile.description,
      eventIds: applied.map((entry) => entry.event.id),
      actionIds: applied.flatMap((entry) => entry.actions.map((action) => action.id)),
    };
    try {
      await persist([...packs.value.filter((entry) => entry.id !== pack.id), pack], pack.id);
    } catch (failure) {
      error.value = errorMessage(failure);
      throw failure;
    }
  };

  const switchProfile = async (id: string, universe: ProfileUniverse): Promise<void> => {
    const resolved = resolvedPacks(packs.value, universe.eventIds, universe.actionIds);
    const target = resolved.find((pack) => pack.id === id);
    if (!target) throw new Error(`unknown profile ${id}`);
    error.value = null;
    const liveEvents = new Set(target.eventIds);
    const liveActions = new Set(target.actionIds);
    const updates: Array<{ kind: 'event' | 'action'; id: string; enabled: boolean }> = [];
    const addUpdate = (kind: 'event' | 'action', recordId: string): void => {
      updates.push({
        kind,
        id: recordId,
        enabled: kind === 'event' ? liveEvents.has(recordId) : liveActions.has(recordId),
      });
    };
    for (const eventId of universe.eventIds) addUpdate('event', eventId);
    for (const actionId of universe.actionIds) addUpdate('action', actionId);
    const knownEvents = new Set(universe.eventIds);
    const knownActions = new Set(universe.actionIds);
    for (const pack of packs.value) {
      for (const eventId of pack.eventIds) {
        if (!knownEvents.has(eventId)) addUpdate('event', eventId);
      }
      for (const actionId of pack.actionIds) {
        if (!knownActions.has(actionId)) addUpdate('action', actionId);
      }
    }
    const pruned = new Set<string>();
    try {
      const result = await control.call<{
        updated: number;
        missing: Array<{ kind: string; id: string }>;
      }>('automation.set_enablement', { updates });
      for (const entry of result.missing ?? []) {
        pruned.add(`${entry.kind}:${entry.id}`);
      }
    } catch (failure) {
      error.value = errorMessage(failure);
      throw failure;
    }
    const drop = (ids: string[], kind: 'event' | 'action'): string[] =>
      ids.filter((entry) => !pruned.has(`${kind}:${entry}`));
    const next = packs.value.map((pack) => ({
      ...pack,
      eventIds: drop(pack.eventIds, 'event'),
      actionIds: drop(pack.actionIds, 'action'),
    }));
    try {
      await persist(next, id);
    } catch (failure) {
      error.value = errorMessage(failure);
      throw failure;
    }
  };

  const createPack = async (name: string): Promise<ProfilePack> => {
    error.value = null;
    const base = slugProfileId(name);
    if (!base) throw new Error('profile names need a letter or digit');
    let id = base;
    let counter = 2;
    while (packs.value.some((pack) => pack.id === id)) {
      const suffix = `-${counter}`;
      id = `${base.slice(0, 64 - suffix.length)}${suffix}`;
      counter += 1;
    }
    const pack: ProfilePack = { id, name: name.trim().slice(0, 80), description: '', eventIds: [], actionIds: [] };
    try {
      await persist([...packs.value, pack], activeId.value);
    } catch (failure) {
      error.value = errorMessage(failure);
      throw failure;
    }
    return pack;
  };

  const deletePack = async (id: string): Promise<void> => {
    if (id === DEFAULT_PROFILE_ID) throw new Error('the default profile cannot be deleted');
    const target = packs.value.find((pack) => pack.id === id);
    if (!target) throw new Error(`unknown profile ${id}`);
    error.value = null;
    const remove = async (kind: 'event' | 'action', recordId: string): Promise<void> => {
      try {
        await control.call('automation.delete', { id: recordId, kind });
      } catch (failure) {
        if (failure instanceof ControlCallError && failure.code === 'automation_not_found') return;
        throw failure;
      }
    };
    try {
      for (const eventId of target.eventIds) {
        await remove('event', eventId);
      }
      for (const actionId of target.actionIds) {
        await remove('action', actionId);
      }
      await persist(
        packs.value.filter((pack) => pack.id !== id),
        activeId.value === id ? DEFAULT_PROFILE_ID : activeId.value,
      );
    } catch (failure) {
      error.value = errorMessage(failure);
      throw failure;
    }
  };

  const adoptRule = async (kind: 'event' | 'action', id: string, profileId = activeId.value): Promise<boolean> => {
    if (profileId === DEFAULT_PROFILE_ID || !packs.value.some((pack) => pack.id === profileId)) return true;
    if (packs.value.some((pack) => (kind === 'event' ? pack.eventIds : pack.actionIds).includes(id))) return true;
    const next = packs.value.map((pack) =>
      pack.id === profileId
        ? {
            ...pack,
            eventIds: kind === 'event' ? [...pack.eventIds, id] : pack.eventIds,
            actionIds: kind === 'action' ? [...pack.actionIds, id] : pack.actionIds,
          }
        : pack,
    );
    try {
      await persist(next, activeId.value);
      return true;
    } catch (failure) {
      error.value = errorMessage(failure);
      return false;
    }
  };

  const setError = (message: string | null): void => {
    error.value = message;
  };

  const setNotice = (message: string | null): void => {
    notice.value = message;
  };

  return {
    packs,
    activeId,
    loading,
    error,
    notice,
    loadProfiles,
    registerPack,
    switchProfile,
    createPack,
    deletePack,
    adoptRule,
    setError,
    setNotice,
  };
}
