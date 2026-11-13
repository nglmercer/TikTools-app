import { contributionFieldsForTrigger } from '../../../../automation/autocomplete-registry.ts';
import { allRegistryFields, fieldsForEventType } from '../../../../automation/event-registry.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { normalizeLocale } from '@/shared/i18n/supported-locales.ts';

export interface TemplateVariable {
  value: string;
  label: string;
  detail?: string;
}

export type VariableScope = 'all' | 'identity' | 'text' | 'file';

const MAX_VARIABLES = 60;
const IDENTITY_KEYS = /^(uniqueid|userid|nickname|username|roomid|creator|user)$/;
const TEXT_KEYS = /^(comment|message|text|nickname|giftname|action|reason|currencyname|name)$/;
const FILE_KEYS = /(?:path|file|sound|audio|url|uri|asset|link|endpoint|webhook)/;

export function variableScopeFor(fieldName: string): VariableScope {
  const key = fieldName.toLowerCase();
  if (key.includes('url') || key.includes('link') || key.includes('endpoint') || key.includes('webhook')) return 'all';
  if (key.includes('uniqueid') || key.includes('viewer') || key.includes('user') || key === 'key' || key === 'type') return 'identity';
  if (key.includes('file') || key.includes('sound') || key.includes('audio')) return 'file';
  if (key.includes('comment') || key.includes('message') || key.includes('text')) return 'text';
  return 'all';
}

function inScope(path: string, scope: VariableScope): boolean {
  if (scope === 'all') return true;
  const leaf = path.split('.').pop()?.toLowerCase() ?? '';
  if (scope === 'identity') return IDENTITY_KEYS.test(leaf);
  if (scope === 'text') return TEXT_KEYS.test(leaf);
  return FILE_KEYS.test(leaf);
}

export function templateVariables(options: {
  trigger?: string;
  scope: VariableScope;
  locale: string;
  globals?: Readonly<Record<string, string>>;
}): TemplateVariable[] {
  const locale = normalizeLocale(options.locale) ?? 'en';
  const lang = locale === 'es' ? 'es' : 'en';
  const label = (field: { path: string; label: { en: string; es: string }; i18key?: string }) => {
    const key = field.i18key ?? `automation.event.field.${field.path}.label`;
    return i18n.global.te(key, locale) ? i18n.global.t(key, {}, { locale }) : field.label[lang];
  };
  const fields = options.trigger ? fieldsForEventType(options.trigger) : allRegistryFields();
  const seen = new Set<string>();
  const variables: TemplateVariable[] = [];
  const push = (variable: TemplateVariable): void => {
    if (seen.has(variable.value) || variables.length >= MAX_VARIABLES) return;
    seen.add(variable.value);
    variables.push(variable);
  };
  for (const field of fields) {
    if (field.path.includes('.0.') || !inScope(field.path, options.scope)) continue;
    push({ value: field.path, label: label(field), detail: field.path });
  }
  for (const field of contributionFieldsForTrigger(options.trigger)) {
    if (inScope(field.path, options.scope)) push({ value: field.path, label: label(field), detail: field.path });
  }
  for (const key of Object.keys(options.globals ?? {}).sort((left, right) => left.localeCompare(right))) {
    push({ value: `globals.${key}`, label: key, detail: `globals.${key}` });
  }
  return variables;
}
