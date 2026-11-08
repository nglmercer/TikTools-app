import { defineStore } from 'pinia';
import { computed } from 'vue';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import type { AutomationRecordVersion, AutomationWriteBase } from '@/entities/automation/model/record-versions.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { canonicalJson } from '@/shared/lib/canonical-json.ts';
import type { AppliedRuleTemplate } from '../lib/rule-templates.ts';
import { buildProfileDoc, resolvedPacks, slugProfileId, useRuleProfiles, type RuleProfile } from './rule-profiles-state.ts';
import { libraryJournal } from './library-journal.ts';

export const useRuleProfilesStore = defineStore('rule-profiles', () => {
  const client = useControlClient();
  const automation = useAutomationStore();
  const profiles = useRuleProfiles(client, () => automation.persistent);
  const library = libraryJournal(client);
  const memory = computed(() => automation.persistent === false);
  const loading = computed(() => memory.value ? profiles.loading.value : library.snapshot.value === null);
  const packs = computed(() => memory.value ? profiles.packs.value : library.snapshot.value?.catalog.packs ?? []);
  const activeId = computed(() => memory.value ? profiles.activeId.value : library.snapshot.value?.catalog.activeId ?? 'default');
  const context = () => library.snapshot.value;

  function universe() {
    return {
      eventIds: automation.behavior.events.map((event) => event.id),
      actionIds: automation.behavior.actions.map((action) => action.id),
    };
  }

  async function guarded(work: () => Promise<void>): Promise<void> {
    profiles.setError(null);
    try {
      await work();
    } catch (failure) {
      if (!(failure instanceof ControlCallError && failure.code === 'cancelled')) profiles.setError(errorMessage(failure));
      throw failure;
    }
  }

  const applyProfile = (profile: RuleProfile, applied: AppliedRuleTemplate[]): Promise<void> =>
    guarded(async () => {
      if (!memory.value) await library.request({ operation: 'import-profile', profile: { id: profile.id, name: profile.name, description: profile.description }, entries: applied }, context());
      else { await automation.createProfileRecords(applied); await profiles.registerPack(profile, applied); }
      await automation.refresh();
    });

  const switchProfile = (id: string): Promise<void> =>
    guarded(async () => {
      if (!memory.value) await library.request({ operation: 'switch-profile', id }, context());
      else await profiles.switchProfile(id, universe());
      await automation.refresh();
    });

  const createProfile = (name: string): Promise<void> =>
    guarded(async () => {
      if (memory.value) await profiles.createPack(name);
      else {
        const base = slugProfileId(name);
        if (!base) throw new Error('profile names need a letter or digit');
        let id = base; let counter = 2;
        while (packs.value.some((pack) => pack.id === id)) { const suffix = `-${counter++}`; id = `${base.slice(0, 64 - suffix.length)}${suffix}`; }
        await library.request({ operation: 'create-profile', id, name: name.trim().slice(0, 80), description: '' }, context());
      }
    });

  const deleteProfile = (id: string): Promise<void> =>
    guarded(async () => {
      if (!memory.value) await library.request({ operation: 'delete-profile', id }, context());
      else await profiles.deletePack(id);
      await automation.refresh();
    });

  async function exportProfile(id: string): Promise<void> {
    profiles.setNotice(null);
    const ids = universe();
    const pack = resolvedPacks(packs.value, ids.eventIds, ids.actionIds).find((entry) => entry.id === id);
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

  async function assignSaved(kind: 'action' | 'event', saved: AutomationRecordVersion, profile: string): Promise<void> {
    if (profile === 'default') return;
    const failed = (failure: unknown) => profiles.setError(i18n.global.t('definitionRecovery.profileFailed', { reason: errorMessage(failure) }));
    try {
      // A presentation refresh may have been superseded by a newer background
      // read. Capture this distinct review from a read started after the save,
      // never from the currently displayed projection or an older request.
      const current = await library.readSnapshot();
      const id = saved.record?.id;
      const source = (kind === 'action' ? current.definitions.actions : current.definitions.events).find((record) => record.id === id);
      const version = current.resourceVersions.find((entry) => entry.scope === kind && entry.id === id);
      if (!saved.host || current.hostId !== saved.host.hostId || current.hostInstanceId !== saved.host.hostInstanceId ||
          !id || !source || version?.revision !== saved.revision || canonicalJson(source) !== canonicalJson(saved.record)) {
        throw new ControlCallError('revision_conflict', 'Read and review the saved definition before assigning its profile.');
      }
      library.installSnapshot(current);
      if (current.catalog.packs.some((pack) => (kind === 'action' ? pack.actionIds : pack.eventIds).includes(id))) return;
      if (!current.catalog.packs.some((pack) => pack.id === profile)) throw new ControlCallError('not_found', 'The originally selected profile is no longer available.');
      void library.request({ operation: 'assign-record', kind, id, profileId: profile }, current).catch(failed);
    } catch (failure) { failed(failure); }
  }
  async function saveAction(action: LiveAction, base: AutomationWriteBase): Promise<AutomationRecordVersion> {
    const profile = activeId.value;
    const result = await automation.saveRecordAndRefresh('action', action, base);
    if (!memory.value) { await assignSaved('action', result, profile); return result; }
    try {
      if (!await profiles.adoptRule('action', action.id, profile)) profiles.setError(i18n.global.t('definitionRecovery.profileFailed', { reason: profiles.error.value ?? '' }));
    }
    catch (failure) { profiles.setError(i18n.global.t('definitionRecovery.profileFailed', { reason: errorMessage(failure) })); }
    return result;
  }

  async function saveEvent(event: LiveEvent, base: AutomationWriteBase): Promise<AutomationRecordVersion> {
    const profile = activeId.value;
    const result = await automation.saveRecordAndRefresh('event', event, base);
    if (!memory.value) { await assignSaved('event', result, profile); return result; }
    try {
      if (!await profiles.adoptRule('event', event.id, profile)) profiles.setError(i18n.global.t('definitionRecovery.profileFailed', { reason: profiles.error.value ?? '' }));
    }
    catch (failure) { profiles.setError(i18n.global.t('definitionRecovery.profileFailed', { reason: errorMessage(failure) })); }
    return result;
  }

  return {
    packs,
    activeId,
    loading,
    error: profiles.error,
    notice: profiles.notice,
    loadProfiles: async () => { if (memory.value) await profiles.loadProfiles(); else { profiles.setError(null); try { await library.refreshSnapshot(); await library.refresh(); } catch (failure) { profiles.setError(errorMessage(failure)); } } },
    applyProfile,
    switchProfile,
    createProfile,
    deleteProfile,
    exportProfile,
    saveAction,
    saveEvent,
  };
});
