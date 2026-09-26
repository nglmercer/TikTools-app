/**
 * Rule profiles: named packs of templates applied as one rule set, with one
 * pack active at a time. Pack membership lives in host app.state
 * (`behavior.profiles.packs` + `behavior.profiles.active`) as a sidecar:
 * records carry no profile column, so switching tolerates hand-deleted
 * rules (missing ids prune silently). The `default` pack always exists and
 * owns every rule outside the stored packs; rules saved by hand while
 * another pack is active are adopted into it. Deleting a pack deletes its
 * rules; only `default` refuses deletion. Pure helpers never touch the DOM
 * except `downloadTextFile`, which the views call on user gesture.
 */

import { ref } from 'vue';

import type { JsonObject } from '../../automation/types.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError, errorMessage } from '../platform/control-client.ts';
import {
  applyRuleTemplate,
  parseRuleTemplate,
  ruleParamDefaults,
  RULE_TEMPLATE_ID_PATTERN,
  type AppliedRuleTemplate,
  type RuleTemplate,
} from '../views/behavior/rule-templates.ts';

export const PROFILE_VERSION = 1;
export const MAX_PROFILE_ENTRIES = 32;
/** Reserved pack owning every rule outside the stored packs. Never deleted. */
export const DEFAULT_PROFILE_ID = 'default';
const MAX_PROFILE_TEXT_BYTES = 262144;

const PACKS_KEY = 'behavior.profiles.packs';
const ACTIVE_KEY = 'behavior.profiles.active';

export interface RuleProfileEntry {
  template: RuleTemplate;
  params: JsonObject;
}

export interface RuleProfile {
  profileVersion: 1;
  id: string;
  name: string;
  description: string;
  params: JsonObject;
  templates: RuleProfileEntry[];
}

export interface ProfilePack {
  id: string;
  name: string;
  description: string;
  eventIds: string[];
  actionIds: string[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function readText(value: unknown, max: number): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (trimmed === '') return null;
  return trimmed.slice(0, max);
}

function readParams(value: unknown): JsonObject | null {
  if (value === undefined) return {};
  if (!isRecord(value)) return null;
  return value as JsonObject;
}

/**
 * Validates unknown JSON into a normalized profile. Never throws:
 * malformed input yields a display-safe error list instead.
 */
export function parseRuleProfile(value: unknown): { ok: true; profile: RuleProfile } | { ok: false; errors: string[] } {
  const errors: string[] = [];
  if (!isRecord(value)) return { ok: false, errors: ['profile must be a JSON object'] };
  if (value['profileVersion'] !== PROFILE_VERSION) {
    return { ok: false, errors: [`unsupported profileVersion (want ${PROFILE_VERSION})`] };
  }
  const id = readText(value['id'], 64);
  if (!id || !RULE_TEMPLATE_ID_PATTERN.test(id)) {
    errors.push('id must match [a-z0-9][a-z0-9._-]{1,63}');
  }
  const name = readText(value['name'], 80);
  if (!name) errors.push('name is required');
  const description = readText(value['description'], 500) ?? '';
  const params = readParams(value['params']);
  if (!params) errors.push('params must be an object');

  const templates: RuleProfileEntry[] = [];
  const raw = value['templates'];
  if (!Array.isArray(raw)) {
    errors.push('templates must be an array');
  } else if (raw.length === 0 || raw.length > MAX_PROFILE_ENTRIES) {
    errors.push(`templates must hold 1-${MAX_PROFILE_ENTRIES} entries`);
  } else {
    raw.forEach((entry: unknown, index: number) => {
      const label = `#${index + 1}`;
      if (!isRecord(entry)) {
        errors.push(`templates[${label}] must be an object`);
        return;
      }
      const parsed = parseRuleTemplate(entry['template']);
      if (!parsed.ok) {
        for (const error of parsed.errors) errors.push(`templates[${label}]: ${error}`);
        return;
      }
      const entryParams = readParams(entry['params']);
      if (!entryParams) {
        errors.push(`templates[${label}].params must be an object`);
        return;
      }
      templates.push({ template: parsed.template, params: entryParams });
    });
  }

  if (errors.length > 0 || !id || !name || !params) {
    return { ok: false, errors };
  }
  return { ok: true, profile: { profileVersion: 1, id, name, description, params, templates } };
}

/** Rejects oversized payloads before parsing. */
export function parseRuleProfileImport(text: string): { profile: RuleProfile | null; errors: string[] } {
  if (text.length > MAX_PROFILE_TEXT_BYTES) {
    return { profile: null, errors: ['import is too large (max 256 KB of text)'] };
  }
  let value: unknown;
  try {
    value = JSON.parse(text) as unknown;
  } catch {
    return { profile: null, errors: ['import is not valid JSON'] };
  }
  if (isRecord(value) && value['templateVersion'] !== undefined) {
    return { profile: null, errors: ['this is a template document; import it as a template'] };
  }
  const parsed = parseRuleProfile(value);
  if (!parsed.ok) return { profile: null, errors: parsed.errors };
  return { profile: parsed.profile, errors: [] };
}

/** Param layers outer-to-inner (mirrors the CLI minus --param): schema defaults → profile → entry. */
export function mergedEntryParams(profile: RuleProfile, entry: RuleProfileEntry): JsonObject {
  return { ...ruleParamDefaults(entry.template.params), ...profile.params, ...entry.params };
}

/** Instantiates every entry with fresh record ids and merged params. */
export function instantiateProfile(profile: RuleProfile): AppliedRuleTemplate[] {
  return profile.templates.map((entry) => applyRuleTemplate(entry.template, mergedEntryParams(profile, entry)));
}

/**
 * Stored packs plus the computed `default` pack first: every event/action
 * outside the stored packs belongs to it, so the header select always has
 * a live option even before the first import.
 */
export function resolvedPacks(
  packs: ProfilePack[],
  eventIds: string[],
  actionIds: string[],
): ProfilePack[] {
  const ownedEvents = new Set(packs.flatMap((pack) => pack.eventIds));
  const ownedActions = new Set(packs.flatMap((pack) => pack.actionIds));
  return [
    {
      id: DEFAULT_PROFILE_ID,
      name: '',
      description: '',
      eventIds: eventIds.filter((id) => !ownedEvents.has(id)),
      actionIds: actionIds.filter((id) => !ownedActions.has(id)),
    },
    ...packs,
  ];
}

/** `My Pack!` → `my-pack`; callers ensure uniqueness against stored packs. */
export function slugProfileId(name: string): string | null {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64);
  if (slug === '' || slug === DEFAULT_PROFILE_ID || !RULE_TEMPLATE_ID_PATTERN.test(slug)) return null;
  return slug;
}

export interface ExportedProfileDoc {
  doc: Record<string, unknown>;
  entries: number;
  skipped: number;
}

/**
 * Serializes a pack back to a `.tikprofile.json` document, mirroring the
 * CLI `template profile-export` shape (one entry per event, actions joined
 * through `actionIds`, events without actions skipped).
 */
export function buildProfileDoc(
  pack: ProfilePack,
  events: Array<{ id: string; name: string; enabled: boolean; trigger: string; filters: unknown; cooldownMs: number; cooldownScope: string; runMode: string; actionIds: string[] }>,
  actions: Array<{ id: string; name: string; typeId: string; enabled: boolean; config: unknown }>,
): ExportedProfileDoc {
  const eventSet = new Set(pack.eventIds);
  const actionSet = new Set(pack.actionIds);
  const byId = new Map(actions.map((action) => [action.id, action]));
  const templates: unknown[] = [];
  let skipped = 0;
  for (const event of events) {
    if (!eventSet.has(event.id)) continue;
    const exported = event.actionIds
      .filter((id) => actionSet.has(id))
      .map((id) => byId.get(id))
      .filter((action) => action !== undefined)
      .map((action) => ({
        name: action.name,
        typeId: action.typeId,
        enabled: action.enabled,
        config: action.config ?? {},
      }));
    if (exported.length === 0) {
      skipped += 1;
      continue;
    }
    templates.push({
      template: {
        templateVersion: 1,
        id: `export-${event.id}`.slice(0, 64),
        title: event.name,
        description: '',
        icon: 'plugin',
        actions: exported,
        event: {
          name: event.name,
          enabled: event.enabled,
          trigger: event.trigger,
          filters: event.filters ?? [],
          cooldownMs: event.cooldownMs,
          cooldownScope: event.cooldownScope,
          runMode: event.runMode,
        },
      },
      params: {},
    });
  }
  return {
    doc: {
      profileVersion: 1,
      id: pack.id,
      name: pack.name === '' ? pack.id : pack.name,
      description: pack.description,
      params: {},
      templates,
    },
    entries: templates.length,
    skipped,
  };
}

/** Triggers a browser download of a profile document (views only). */
export function downloadTextFile(filename: string, text: string): void {
  if (typeof document === 'undefined' || typeof URL === 'undefined') return;
  const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function asPack(value: unknown): ProfilePack | null {
  if (!isRecord(value)) return null;
  const id = readText(value['id'], 64);
  const name = readText(value['name'], 80);
  const eventIds = value['eventIds'];
  const actionIds = value['actionIds'];
  if (!id || !name || !Array.isArray(eventIds) || !Array.isArray(actionIds)) return null;
  if (!eventIds.every((entry) => typeof entry === 'string') || !actionIds.every((entry) => typeof entry === 'string')) {
    return null;
  }
  return {
    id,
    name,
    description: typeof value['description'] === 'string' ? (value['description'] as string).slice(0, 500) : '',
    eventIds: [...eventIds] as string[],
    actionIds: [...actionIds] as string[],
  };
}

function asPacks(raw: string | undefined): { packs: ProfilePack[]; errors: string[] } {
  if (!raw || raw.trim() === '') return { packs: [], errors: [] };
  let value: unknown;
  try {
    value = JSON.parse(raw) as unknown;
  } catch {
    return { packs: [], errors: ['stored profiles are not valid JSON'] };
  }
  if (!Array.isArray(value)) return { packs: [], errors: ['stored profiles must be an array'] };
  const packs: ProfilePack[] = [];
  const errors: string[] = [];
  value.forEach((entry: unknown, index: number) => {
    const pack = asPack(entry);
    if (!pack) {
      errors.push(`stored profiles[${index + 1}] is malformed (dropped)`);
    } else if (pack.id === DEFAULT_PROFILE_ID) {
      errors.push(`stored profiles[${index + 1}] uses the reserved default id (dropped)`);
    } else {
      packs.push(pack);
    }
  });
  return { packs, errors };
}

export interface ProfileUniverse {
  eventIds: string[];
  actionIds: string[];
}

export function useRuleProfiles(control: ControlClient) {
  const packs = ref<ProfilePack[]>([]);
  const activeId = ref<string>(DEFAULT_PROFILE_ID);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const loadProfiles = async (): Promise<void> => {
    loading.value = true;
    error.value = null;
    try {
      const result = await control.call<{ state: Record<string, string> }>('app.state.get', {
        keys: [PACKS_KEY, ACTIVE_KEY],
      });
      const parsed = asPacks(result.state[PACKS_KEY]);
      packs.value = parsed.packs;
      if (parsed.errors.length > 0) error.value = parsed.errors.join('; ');
      const active = (result.state[ACTIVE_KEY] ?? '').trim();
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
    await control.call('app.state.set', { key: PACKS_KEY, value: JSON.stringify(next) });
    await control.call('app.state.set', { key: ACTIVE_KEY, value: active });
    packs.value = next;
    activeId.value = active;
  };

  /**
   * Registers an applied profile (replaces the same-id pack; the old
   * records stay but become unmanaged) and marks it active. Call after
   * the records exist; ids come from the applied entries.
   */
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

  const setEnabled = async (kind: 'event' | 'action', id: string, enabled: boolean, pruned: Set<string>): Promise<void> => {
    try {
      await control.call(enabled ? 'automation.enable' : 'automation.disable', { id, kind });
    } catch (failure) {
      if (failure instanceof ControlCallError && failure.code === 'automation_not_found') {
        pruned.add(`${kind}:${id}`);
        return;
      }
      throw failure;
    }
  };

  /**
   * Makes one pack live: enables its records, disables every other known
   * record. The universe (all record ids) comes from the caller snapshot;
   * the `default` target resolves to every id outside the stored packs.
   * Ids whose records were hand-deleted prune silently. The active marker
   * only advances on success.
   */
  const switchProfile = async (id: string, universe: ProfileUniverse): Promise<void> => {
    const resolved = resolvedPacks(packs.value, universe.eventIds, universe.actionIds);
    const target = resolved.find((pack) => pack.id === id);
    if (!target) throw new Error(`unknown profile ${id}`);
    error.value = null;
    const liveEvents = new Set(target.eventIds);
    const liveActions = new Set(target.actionIds);
    const pruned = new Set<string>();
    try {
      for (const eventId of universe.eventIds) {
        await setEnabled('event', eventId, liveEvents.has(eventId), pruned);
      }
      for (const actionId of universe.actionIds) {
        await setEnabled('action', actionId, liveActions.has(actionId), pruned);
      }
      // Stored ids missing from the snapshot are either stale (404 → prune)
      // or fresher than the snapshot (toggled into place either way).
      const knownEvents = new Set(universe.eventIds);
      const knownActions = new Set(universe.actionIds);
      for (const pack of packs.value) {
        for (const eventId of pack.eventIds) {
          if (!knownEvents.has(eventId)) await setEnabled('event', eventId, liveEvents.has(eventId), pruned);
        }
        for (const actionId of pack.actionIds) {
          if (!knownActions.has(actionId)) await setEnabled('action', actionId, liveActions.has(actionId), pruned);
        }
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

  /** Creates an empty pack (unique slug id); the active pack stays put. */
  const createPack = async (name: string): Promise<ProfilePack> => {
    error.value = null;
    const base = slugProfileId(name);
    if (!base) throw new Error('profile names need a letter or digit');
    let id = base;
    let counter = 2;
    while (packs.value.some((pack) => pack.id === id)) {
      id = `${base}-${counter}`.slice(0, 64);
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

  /**
   * Deletes a pack and every rule it owns (events and actions); missing
   * records are tolerated. Refuses `default`; falls back to it when the
   * active pack goes away.
   */
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

  /**
   * Adopts a hand-saved rule into the active pack (no-op when the rule
   * already belongs to a pack or the active pack is `default`, which owns
   * leftovers by definition).
   */
  const adoptRule = async (kind: 'event' | 'action', id: string): Promise<void> => {
    if (activeId.value === DEFAULT_PROFILE_ID) return;
    if (packs.value.some((pack) => pack.eventIds.includes(id) || pack.actionIds.includes(id))) return;
    const next = packs.value.map((pack) =>
      pack.id === activeId.value
        ? {
            ...pack,
            eventIds: kind === 'event' ? [...pack.eventIds, id] : pack.eventIds,
            actionIds: kind === 'action' ? [...pack.actionIds, id] : pack.actionIds,
          }
        : pack,
    );
    try {
      await persist(next, activeId.value);
    } catch (failure) {
      error.value = errorMessage(failure);
    }
  };

  const setError = (message: string): void => {
    error.value = message;
  };

  return {
    packs,
    activeId,
    loading,
    error,
    loadProfiles,
    registerPack,
    switchProfile,
    createPack,
    deletePack,
    adoptRule,
    setError,
  };
}
