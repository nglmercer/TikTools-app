import { expect, test, vi } from 'vitest';
import type { JsonValue } from '../../../../automation/types.ts';
import { applyRuleTemplate, parseRuleTemplate, substituteRuleParams, type RuleTemplate } from './rule-templates.ts';
import { instantiateProfile, type RuleProfile } from '../model/rule-profiles-state.ts';
import { TEMPLATE_AGGREGATE_BYTES, TEMPLATE_RECORD_BYTES } from './bounded-expansion.ts';

function template(value: JsonValue, actions = 1): RuleTemplate {
  const parsed = parseRuleTemplate({ templateVersion: 1, id: 'bounded-template', title: 'Bounded',
    actions: Array.from({ length: actions }, (_, index) => ({ name: `Action ${index}`, typeId: 'core.log', config: { message: value } })),
    event: { name: 'Chat', trigger: 'tiktok.chat' } });
  if (!parsed.ok) throw new Error(parsed.errors.join('; '));
  return parsed.template;
}
function profile(value: string): RuleProfile {
  const shared = template('{{ params.value }}', 4);
  return { profileVersion: 1, id: 'bounded-profile', name: 'Bounded', description: '', params: { value },
    templates: Array.from({ length: 32 }, () => ({ template: shared, params: {} })) };
}
test('small template plus shared parameter cannot allocate hundreds of megabytes through repeated inline spans', () => {
  const source = template('{{ params.value }}'.repeat(500)); const params = { value: 'x'.repeat(TEMPLATE_RECORD_BYTES) };
  expect(JSON.stringify(source).length).toBeLessThan(16_000);
  const stringify = vi.spyOn(JSON, 'stringify'); let failure: unknown; let serialized: number;
  try { applyRuleTemplate(source, params); } catch (error) { failure = error; } finally { serialized = stringify.mock.calls.length; stringify.mockRestore(); }
  expect(failure).toMatchObject({ code: 'too_large' });
  expect(serialized).toBe(0);
});
test('inline object text is measured with outer escaping before serialization, retaining valid scalar semantics', () => {
  const raw = { quoted: '"'.repeat(70_000), unicode: '🌹', control: '\n' };
  const stringify = vi.spyOn(JSON, 'stringify'); let failure: unknown; let serialized: number;
  try { substituteRuleParams('value={{ params.raw }}', { raw }); } catch (error) { failure = error; } finally { serialized = stringify.mock.calls.length; stringify.mockRestore(); }
  expect(failure).toMatchObject({ code: 'too_large' }); expect(serialized).toBe(0);
  const valid = { quoted: '"🌹\n', nested: [1, false, null] };
  expect(substituteRuleParams('value={{ params.raw }}', { raw: valid })).toBe(`value=${JSON.stringify(valid)}`);
});
test('sole objects are measured before cloning and raw params are not recursively expanded', () => {
  const clone = vi.spyOn(Object, 'fromEntries'); let failure: unknown; let cloned: number;
  try { substituteRuleParams('{{ params.raw }}', { raw: { message: 'x'.repeat(TEMPLATE_RECORD_BYTES) } }); }
  catch (error) { failure = error; } finally { cloned = clone.mock.calls.length; clone.mockRestore(); }
  expect(failure).toMatchObject({ code: 'too_large' }); expect(cloned).toBe(0);
  const params = JSON.parse('{"raw":{"__proto__":{"safe":true},"message":"{{ params.later }}"},"later":"must not expand"}');
  const result = substituteRuleParams('{{ params.raw }}', params);
  expect(result).toEqual(params.raw); expect(result).not.toBe(params.raw); expect(Object.hasOwn(result as object, '__proto__')).toBe(true);
});
test('profile aggregate is checked before any expanded tree is materialized, even when every individual record fits', () => {
  const source = profile('x'.repeat(70_000));
  expect(JSON.stringify(source).length).toBeLessThan(256 * 1024);
  expect(() => applyRuleTemplate(source.templates[0]!.template, source.params)).not.toThrow();
  const clone = vi.spyOn(Object, 'fromEntries'); let failure: unknown; let cloned: number;
  try { instantiateProfile(source); } catch (error) { failure = error; } finally { cloned = clone.mock.calls.length; clone.mockRestore(); }
  expect(failure).toMatchObject({ code: 'too_large' }); expect(cloned).toBe(0);
});
test('UTF-8 and JSON escapes enforce the exact record boundary and the shared aggregate without truncation', () => {
  const exact = `${'🌹'.repeat(65_535)}xx`;
  expect(new TextEncoder().encode(JSON.stringify(exact)).length).toBe(TEMPLATE_RECORD_BYTES);
  expect(substituteRuleParams('{{ params.value }}', { value: exact })).toBe(exact);
  expect(() => substituteRuleParams('{{ params.value }}', { value: `${exact}x` })).toThrow();
  expect(substituteRuleParams('{{ params.value }}', { value: '\n'.repeat(131_071) })).toHaveLength(131_071);
  expect(() => substituteRuleParams('{{ params.value }}', { value: '\n'.repeat(131_072) })).toThrow();
  expect(() => instantiateProfile(profile('🌹'.repeat(20_000)))).toThrow();
  const valid = instantiateProfile(profile('🌹'.repeat(15_000)));
  expect(valid).toHaveLength(32); expect(new TextEncoder().encode(JSON.stringify(valid)).length).toBeLessThan(TEMPLATE_AGGREGATE_BYTES);
  expect(valid[31]?.actions[3]?.config.message).toBe('🌹'.repeat(15_000));
});
test('sole and inline param traversal enforce structural budgets and exact numeric input', () => {
  let deep: JsonValue = 'leaf'; for (let i = 0; i < 65; ++i) deep = { nested: deep };
  expect(() => substituteRuleParams('{{ params.raw }}', { raw: deep })).toThrow();
  expect(() => substituteRuleParams('raw={{ params.raw }}', { raw: deep })).toThrow();
  expect(() => substituteRuleParams('{{ params.raw }}', { raw: Array(65_536).fill(0) })).toThrow();
  expect(() => substituteRuleParams('raw={{ params.raw }}', { raw: 9007199254740992 })).toThrow();
});
