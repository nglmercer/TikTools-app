import { instantiateBoundedTemplates } from './bounded-expansion.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import { parseRuleTemplate, ruleParamDefaults, RULE_TEMPLATE_ID_PATTERN,
  type AppliedRuleTemplate, type RuleTemplate,
} from './rule-templates.ts';
import { isRecord, readText, readParams } from './profile-values.ts';

export const PROFILE_VERSION = 1;
export const MAX_PROFILE_ENTRIES = 32;
export const DEFAULT_PROFILE_ID = 'default';
export const MAX_PROFILE_TEXT_BYTES = 262144;

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

export interface ProfileUniverse {
  eventIds: string[];
  actionIds: string[];
}

export function parseRuleProfile(value: unknown): { ok: true; profile: RuleProfile } | { ok: false; errors: string[] } {
  const errors: string[] = [];
  if (!isRecord(value)) return { ok: false, errors: ['profile must be a JSON object'] };
  if (value['profileVersion'] !== PROFILE_VERSION) {
    return { ok: false, errors: [`unsupported profileVersion (want ${PROFILE_VERSION})`] };
  }
  const rawId = value['id'];
  const id = typeof rawId === 'string' && rawId !== '' && rawId !== DEFAULT_PROFILE_ID && Array.from(rawId).length <= 64 &&
    rawId.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '') === rawId && !/\p{Cc}/u.test(rawId) ? rawId : null;
  if (!id) {
    errors.push('id must be a nonreserved canonical profile ID of 1–64 characters');
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

export function mergedEntryParams(profile: RuleProfile, entry: RuleProfileEntry, overrides: JsonObject = {}): JsonObject {
  return { ...ruleParamDefaults(entry.template.params), ...profile.params, ...entry.params, ...overrides };
}

export function instantiateProfile(profile: RuleProfile, overrides: JsonObject = {}): AppliedRuleTemplate[] {
  return instantiateBoundedTemplates(profile.templates.map((entry) => ({ template: entry.template, params: mergedEntryParams(profile, entry, overrides) })));
}

export function profileParamSchema(profile: RuleProfile): { schema: JsonObject; defaults: JsonObject } | null {
  const properties: JsonObject = {};
  const required: string[] = [];
  for (const entry of profile.templates) {
    const schema = entry.template.params;
    if (!isRecord(schema)) continue;
    const props = schema['properties'];
    if (isRecord(props)) Object.assign(properties, props);
    const req = schema['required'];
    if (Array.isArray(req)) {
      for (const key of req) {
        if (typeof key === 'string' && !required.includes(key)) required.push(key);
      }
    }
  }
  if (Object.keys(properties).length === 0) return null;
  const schema: JsonObject = { type: 'object', properties };
  if (required.length > 0) schema['required'] = required;
  return { schema, defaults: { ...ruleParamDefaults(schema), ...profile.params } };
}

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
