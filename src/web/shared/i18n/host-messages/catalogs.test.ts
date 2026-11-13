import { afterEach, expect, it, vi } from 'vitest';
import { hostMessageContract } from '../../../../../scripts/lib/host-message-contract.ts';
import { activateLocale, i18n, localizedText, mergePluginMessages } from '../i18n.ts';
import zh from './zh.ts';
import ja from './ja.ts';

afterEach(async () => { await activateLocale('en'); });

for (const [locale, messages] of Object.entries({ zh, ja }) as Array<['zh' | 'ja', Record<string, string>]>) {
  it(`${locale} covers the actual native action descriptors and generated event fields`, async () => {
    const source = hostMessageContract();
    expect(source['automation.action.core.log.title']).toBe('Write to the log');
    expect(source['automation.event.field.event.user.nickname.label']).toBe('Nickname');
    expect(Object.keys(messages).sort()).toEqual(Object.keys(source).sort());
    await activateLocale(locale);
    const failures: string[] = [];
    const logger = vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => { failures.push(args.map(String).join(' ')); });
    try {
      for (const [key, fallback] of Object.entries(source)) {
        expect(i18n.global.te(key, locale), key).toBe(true);
        expect(localizedText({ default: fallback, i18key: key }), key).toBe(messages[key]);
      }
      expect(failures).toEqual([]);
    } finally { logger.mockRestore(); }
    // A normal host snapshot can supply its en/es fallback catalog after activation.
    // It must not erase the loaded CJK strings or change the descriptor identity.
    mergePluginMessages({ en: source });
    expect(localizedText({ default: 'Write to the log', i18key: 'automation.action.core.log.title' }))
      .toBe(locale === 'zh' ? '写入日志' : 'メッセージをログに記録');
  });
}
