import { expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  applyRuleTemplate,
  snapshotRuleTemplates,
  exportRuleTemplate,
  missingRuleTemplateRequirements,
  parseRuleTemplate,
  parseRuleTemplateList,
  ruleParamDefaults,
  substituteRuleParams,
} from '../../../../../../src/web/entities/rule-template/lib/rule-templates.ts';
import { ACTION_TYPE, EVENT_KIND } from '../../../../../shared/index.ts';

const MINIMAL = {
  templateVersion: 1,
  id: 'gift-log',
  title: 'Gift log',
  actions: [{ name: 'Log', typeId: ACTION_TYPE.coreLog, config: { message: 'hi' } }],
  event: { name: 'Gift', trigger: EVENT_KIND.tiktokGift },
};

test('host templates parse, stay available on installed types, and round-trip', () => {
  // The host serves these documents in the snapshot from
  // crates/tiktools-core/src/services/builtin-templates.json; the frontend
  // never hardcodes its own copy anymore.
  const documents = JSON.parse(
    readFileSync(join(process.cwd(), 'crates', 'tiktools-core', 'src', 'services', 'builtin-templates.json'), 'utf8'),
  ) as unknown[];
  const templates = snapshotRuleTemplates({ hostTemplates: documents });
  expect(templates.length).toBeGreaterThanOrEqual(4);
  const available = new Set([ACTION_TYPE.coreFetch, 'core.points', 'core.points.subtract', 'audio.play', ACTION_TYPE.coreLog]);
  for (const template of templates) {
    const requirements = missingRuleTemplateRequirements(
      template,
      available,
      [EVENT_KIND.tiktokGift, EVENT_KIND.tiktokChat],
    );
    expect(requirements).toEqual({ missingTypeIds: [], unknownTrigger: false });
    const reparsed = parseRuleTemplate(JSON.parse(exportRuleTemplate(template)));
    expect(reparsed.ok).toBe(true);
  }
});

test('snapshot templates dedupe by id and drop unparseable entries', () => {
  const minimal = (id: string) => ({
    templateVersion: 1,
    id,
    title: { default: id },
    actions: [{ name: 'Log', typeId: ACTION_TYPE.coreLog, config: {} }],
    event: { name: id, trigger: EVENT_KIND.tiktokChat },
  });
  const templates = snapshotRuleTemplates({
    hostTemplates: [minimal('host-one')],
    pluginTemplates: [minimal('host-one'), minimal('sonicboom.server/chat-tts'), { broken: true }],
  });
  expect(templates.map((template) => template.id)).toEqual(['host-one', 'sonicboom.server/chat-tts']);
});

test('minimal template normalizes defaults', () => {
  const parsed = parseRuleTemplate(MINIMAL);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(parsed.template).toMatchObject({
    id: 'gift-log',
    icon: 'plugin',
    actions: [{ name: 'Log', typeId: ACTION_TYPE.coreLog, enabled: true, config: { message: 'hi' } }],
    event: {
      name: 'Gift',
      enabled: true,
      trigger: EVENT_KIND.tiktokGift,
      filters: [],
      cooldownMs: 0,
      cooldownScope: 'global',
      runMode: 'all',
    },
  });
});

test('malformed templates yield display-safe errors, never throw', () => {
  expect(parseRuleTemplate(null)).toEqual({ ok: false, errors: ['template must be a JSON object'] });
  expect(parseRuleTemplate({ ...MINIMAL, templateVersion: 99 }).ok).toBe(false);
  expect(parseRuleTemplate({ ...MINIMAL, id: '../evil' }).ok).toBe(false);
  expect(parseRuleTemplate({ ...MINIMAL, actions: [] }).ok).toBe(false);
  expect(parseRuleTemplate({ ...MINIMAL, event: { name: 'x' } }).ok).toBe(false);
  const badFilter = parseRuleTemplate({
    ...MINIMAL,
    event: { name: 'x', trigger: EVENT_KIND.tiktokGift, filters: [{ path: 'p', operator: 'nope', value: '' }] },
  });
  expect(badFilter.ok).toBe(false);
});

test('list import accepts a single doc, an array, or {templates}', () => {
  expect(parseRuleTemplateList(MINIMAL).templates).toHaveLength(1);
  expect(parseRuleTemplateList([MINIMAL, { ...MINIMAL, id: 'second' }]).templates).toHaveLength(2);
  expect(parseRuleTemplateList({ templates: [MINIMAL] }).templates).toHaveLength(1);
  const mixed = parseRuleTemplateList([MINIMAL, { templateVersion: 1 }]);
  expect(mixed.templates).toHaveLength(1);
  expect(mixed.errors.length).toBeGreaterThan(0);
});

test('params substitute raw for sole spans and text otherwise, event spans survive', () => {
  expect(substituteRuleParams('{{ params.delta }}', { delta: 10 })).toBe(10);
  expect(substituteRuleParams('n={{ params.delta }}!', { delta: 10 })).toBe('n=10!');
  expect(substituteRuleParams('{{ event.user.uniqueId }}', {})).toBe('{{ event.user.uniqueId }}');
  expect(substituteRuleParams('{{ params.missing }}', {})).toBe('');
  expect(ruleParamDefaults({ type: 'object', properties: { url: { default: 'https://' } } })).toEqual({ url: 'https://' });
});

test('apply assigns fresh ids, links the event, and keeps runtime spans', () => {
  const parsed = parseRuleTemplate({
    ...MINIMAL,
    actions: [{ name: 'P {{ params.delta }}', typeId: 'core.points', config: { delta: '{{ params.delta }}', note: '{{ event.type }}' } }],
  });
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  const first = applyRuleTemplate(parsed.template, { delta: 5 });
  const second = applyRuleTemplate(parsed.template, { delta: 5 }, { eventName: 'Custom' });
  expect(first.actions[0]?.config).toEqual({ delta: 5, note: '{{ event.type }}' });
  expect(first.event.actionIds).toEqual(first.actions.map((action) => action.id));
  expect(first.event.id).not.toBe(second.event.id);
  expect(first.actions[0]?.id).not.toBe(second.actions[0]?.id);
  expect(second.event.name).toBe('Custom');
  expect(first.actions[0]?.name).toBe('P 5');
});

test('shared fixtures agree with the Rust engine (valid vs invalid)', () => {
  const root = join(process.cwd(), 'tests', 'fixtures', 'templates');
  const valid = parseRuleTemplate(JSON.parse(readFileSync(join(root, 'valid-minimal.json'), 'utf8')));
  expect(valid.ok).toBe(true);
  expect(parseRuleTemplate(JSON.parse(readFileSync(join(root, 'invalid-version.json'), 'utf8'))).ok).toBe(false);
  expect(parseRuleTemplate(JSON.parse(readFileSync(join(root, 'invalid-operator.json'), 'utf8'))).ok).toBe(false);
});

test('missing requirements name unavailable types and unknown triggers', () => {
  const parsed = parseRuleTemplate(MINIMAL);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(missingRuleTemplateRequirements(parsed.template, new Set(), [EVENT_KIND.tiktokChat])).toEqual({
    missingTypeIds: [ACTION_TYPE.coreLog],
    unknownTrigger: true,
  });
});

test('builtin param titles carry i18keys that resolve in every shipped locale', async () => {
  // Param labels render through `localizedText` in the schema form, so a
  // key missing from a locale would silently fall back to English. Pin
  // full coverage for the host-supported locales instead.
  const seen = new Map<string, string>();
  const documents = JSON.parse(readFileSync(join(process.cwd(), 'crates', 'tiktools-core', 'src', 'services', 'builtin-templates.json'), 'utf8')) as unknown[];
  for (const template of snapshotRuleTemplates({ hostTemplates: documents })) {
    const properties = template.params?.['properties'];
    if (!properties || typeof properties !== 'object' || Array.isArray(properties)) continue;
    for (const property of Object.values(properties as Record<string, unknown>)) {
      if (!property || typeof property !== 'object') continue;
      const title = (property as Record<string, unknown>)['title'];
      if (!title || typeof title !== 'object') continue;
      const key = (title as Record<string, unknown>)['i18key'];
      if (typeof key === 'string' && key) seen.set(key, (title as Record<string, unknown>)['default'] as string);
    }
  }
  expect(seen.size).toBeGreaterThanOrEqual(6);
  for (const locale of ['en', 'es', 'pt', 'fr', 'zh', 'ja']) {
    const messages = (await import(`../../../../../../src/web/shared/i18n/locales/${locale}.ts`)).default;
    const flat: Record<string, string> = {};
    const walk = (node: Record<string, unknown>, path: string[]) => {
      for (const [k, v] of Object.entries(node)) {
        if (typeof v === 'string') flat[[...path, k].join('.')] = v;
        else if (v && typeof v === 'object') walk(v as Record<string, unknown>, [...path, k]);
      }
    };
    walk(messages as Record<string, unknown>, []);
    for (const key of seen.keys()) {
      expect(flat[key], `${locale} missing ${key}`).toBeTruthy();
    }
  }
});
