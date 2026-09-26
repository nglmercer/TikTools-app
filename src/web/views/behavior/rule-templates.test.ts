import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  applyRuleTemplate,
  BUILTIN_RULE_TEMPLATES,
  exportRuleTemplate,
  missingRuleTemplateRequirements,
  parseRuleTemplate,
  parseRuleTemplateList,
  ruleParamDefaults,
  substituteRuleParams,
} from './rule-templates.ts';

const MINIMAL = {
  templateVersion: 1,
  id: 'gift-log',
  title: 'Gift log',
  actions: [{ name: 'Log', typeId: 'core.log', config: { message: 'hi' } }],
  event: { name: 'Gift', trigger: 'tiktok.gift' },
};

test('builtins parse, stay available on core types, and round-trip', () => {
  expect(BUILTIN_RULE_TEMPLATES.length).toBeGreaterThanOrEqual(4);
  const available = new Set(['core.fetch', 'core.points', 'core.points.subtract', 'audio.play', 'core.log']);
  for (const template of BUILTIN_RULE_TEMPLATES) {
    const requirements = missingRuleTemplateRequirements(
      template,
      available,
      ['tiktok.gift', 'tiktok.chat'],
    );
    expect(requirements).toEqual({ missingTypeIds: [], unknownTrigger: false });
    const reparsed = parseRuleTemplate(JSON.parse(exportRuleTemplate(template)));
    expect(reparsed.ok).toBe(true);
  }
});

test('minimal template normalizes defaults', () => {
  const parsed = parseRuleTemplate(MINIMAL);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(parsed.template).toMatchObject({
    id: 'gift-log',
    icon: 'plugin',
    actions: [{ name: 'Log', typeId: 'core.log', enabled: true, config: { message: 'hi' } }],
    event: {
      name: 'Gift',
      enabled: true,
      trigger: 'tiktok.gift',
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
    event: { name: 'x', trigger: 'tiktok.gift', filters: [{ path: 'p', operator: 'nope', value: '' }] },
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
  const root = join(import.meta.dir, '..', '..', '..', '..', 'examples', 'templates', 'fixtures');
  const valid = parseRuleTemplate(JSON.parse(readFileSync(join(root, 'valid-minimal.json'), 'utf8')));
  expect(valid.ok).toBe(true);
  expect(parseRuleTemplate(JSON.parse(readFileSync(join(root, 'invalid-version.json'), 'utf8'))).ok).toBe(false);
  expect(parseRuleTemplate(JSON.parse(readFileSync(join(root, 'invalid-operator.json'), 'utf8'))).ok).toBe(false);
});

test('missing requirements name unavailable types and unknown triggers', () => {
  const parsed = parseRuleTemplate(MINIMAL);
  expect(parsed.ok).toBe(true);
  if (!parsed.ok) return;
  expect(missingRuleTemplateRequirements(parsed.template, new Set(), ['tiktok.chat'])).toEqual({
    missingTypeIds: ['core.log'],
    unknownTrigger: true,
  });
});
