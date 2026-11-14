import { expect, test } from 'bun:test';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { docLocales } from '../locales.ts';
import { parseRuleTemplate } from '../../src/web/entities/rule-template/lib/rule-templates.ts';
import { parseRuleProfile } from '../../src/web/entities/rule-template/lib/rule-profiles.ts';

test('localized template/profile recipes are accepted by the real import parsers with effects disabled', async () => {
  for (const locale of docLocales) {
    const text = await readFile(resolve(import.meta.dirname, `../content/${locale}/developer/templates.md`), 'utf8');
    const blocks = [...text.split('{#operational-details}')[1]!.matchAll(/```json\n([\s\S]*?)\n```/g)].map((match) => JSON.parse(match[1]!));
    expect(blocks).toHaveLength(2);
    const template = parseRuleTemplate(blocks[0]);
    const profile = parseRuleProfile(blocks[1]);
    expect(template.ok, locale).toBe(true);
    expect(profile.ok, locale).toBe(true);
    if (template.ok) {
      expect(template.template.actions.every((action) => !action.enabled)).toBe(true);
      expect(template.template.event.enabled).toBe(false);
    }
  }
});
