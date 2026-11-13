import { afterEach, describe, expect, it } from 'vitest';
import { templateVariables, variableScopeFor } from './template-variables.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';

afterEach(async () => { await activateLocale('en'); });

describe('template variables', () => {
  it('derives a scope from the field name', () => {
    expect(variableScopeFor('webhookUrl')).toBe('all');
    expect(variableScopeFor('uniqueId')).toBe('identity');
    expect(variableScopeFor('soundFile')).toBe('file');
    expect(variableScopeFor('message')).toBe('text');
    expect(variableScopeFor('payload')).toBe('all');
  });

  it('offers registry paths, filtered by scope, plus globals', () => {
    const all = templateVariables({ trigger: 'tiktok.gift', scope: 'all', locale: 'en', globals: { score: '1' } });
    expect(all.some((variable) => variable.value === 'event.data.giftName')).toBe(true);
    expect(all.at(-1)).toEqual({ value: 'globals.score', label: 'score', detail: 'globals.score' });
    const identity = templateVariables({ trigger: 'tiktok.gift', scope: 'identity', locale: 'es' });
    expect(identity.length).toBeGreaterThan(0);
    expect(identity.every((variable) => /(uniqueId|userId|nickname|username|roomId|creator|user)$/i.test(variable.value))).toBe(true);
  });

  it('never repeats a path', () => {
    const values = templateVariables({ scope: 'all', locale: 'en' }).map((variable) => variable.value);
    expect(new Set(values).size).toBe(values.length);
  });

  for (const locale of ['zh', 'ja'] as const) it(`localizes ${locale} field labels while preserving machine paths and globals`, async () => {
    await activateLocale(locale);
    const values = templateVariables({ trigger: 'tiktok.gift', scope: 'all', locale, globals: { score: '1' } });
    const nickname = values.find((item) => item.value === 'event.user.nickname');
    expect(nickname?.label).toBe(locale === 'zh' ? '昵称' : 'ニックネーム');
    expect(nickname?.detail).toBe('event.user.nickname');
    expect(values.at(-1)).toEqual({ value: 'globals.score', label: 'score', detail: 'globals.score' });
  });
});
