import { defineStore } from 'pinia';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import type { AutomationRecordVersion, AutomationWriteBase } from '@/entities/automation/model/record-versions.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import type { AppliedRuleTemplate } from '../lib/rule-templates.ts';
import { buildProfileDoc, resolvedPacks, useRuleProfiles, type RuleProfile } from './rule-profiles-state.ts';

export const useRuleProfilesStore = defineStore('rule-profiles', () => {
  const client = useControlClient();
  const automation = useAutomationStore();
  const profiles = useRuleProfiles(client);

  function universe() {
    return {
      eventIds: automation.behavior.events.map((event) => event.id),
      actionIds: automation.behavior.actions.map((action) => action.id),
    };
  }

  async function guarded(work: () => Promise<void>): Promise<void> {
    try {
      await work();
    } catch (failure) {
      profiles.setError(errorMessage(failure));
      throw failure;
    }
  }

  const applyProfile = (profile: RuleProfile, applied: AppliedRuleTemplate[]): Promise<void> =>
    guarded(async () => {
      await automation.createProfileRecords(applied);
      await profiles.registerPack(profile, applied);
      await automation.refresh();
    });

  const switchProfile = (id: string): Promise<void> =>
    guarded(async () => {
      await profiles.switchProfile(id, universe());
      await automation.refresh();
    });

  const createProfile = (name: string): Promise<void> =>
    guarded(async () => {
      await profiles.createPack(name);
    });

  const deleteProfile = (id: string): Promise<void> =>
    guarded(async () => {
      await profiles.deletePack(id);
      await automation.refresh();
    });

  async function exportProfile(id: string): Promise<void> {
    profiles.setNotice(null);
    const ids = universe();
    const pack = resolvedPacks(profiles.packs.value, ids.eventIds, ids.actionIds).find((entry) => entry.id === id);
    if (!pack) {
      profiles.setError(`unknown profile ${id}`);
      return;
    }
    const exported = buildProfileDoc(pack, automation.behavior.events, automation.behavior.actions);
    try {
      const result = await client.call<{ ok: boolean; path: string }>('system.saveFile', {
        filename: `${pack.id}.tikprofile.json`,
        content: `${JSON.stringify(exported.doc, null, 2)}\n`,
      });
      profiles.setNotice(i18n.global.t('automations.profileExported', { path: result.path }));
    } catch (failure) {
      profiles.setError(`export failed: ${errorMessage(failure)}`);
    }
  }

  async function saveAction(action: LiveAction, base: AutomationWriteBase): Promise<AutomationRecordVersion> {
    const profile = profiles.activeId.value;
    const result = await automation.saveRecordAndRefresh('action', action, base);
    await profiles.adoptRule('action', action.id, profile);
    return result;
  }

  async function saveEvent(event: LiveEvent, base: AutomationWriteBase): Promise<AutomationRecordVersion> {
    const profile = profiles.activeId.value;
    const result = await automation.saveRecordAndRefresh('event', event, base);
    await profiles.adoptRule('event', event.id, profile);
    return result;
  }

  return {
    packs: profiles.packs,
    activeId: profiles.activeId,
    loading: profiles.loading,
    error: profiles.error,
    notice: profiles.notice,
    loadProfiles: profiles.loadProfiles,
    applyProfile,
    switchProfile,
    createProfile,
    deleteProfile,
    exportProfile,
    saveAction,
    saveEvent,
  };
});
