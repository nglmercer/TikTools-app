import { expect, test } from 'bun:test';
import { tokenizeDocSearch } from './search-tokenizer.ts';

test('indexes Chinese and Japanese compounds without whitespace and preserves API identifiers', () => {
  const zh = tokenizeDocSearch('安装应用程序并创建自动化规则');
  const ja = tokenizeDocSearch('自動化ルールを作成して接続を確認します');
  expect(zh).toContain('安装');
  expect(zh).toContain('自动');
  expect(ja).toContain('自動');
  expect(ja).toContain('接続');
  expect(tokenizeDocSearch('rpc.discover JSON-RPC')).toEqual(expect.arrayContaining(['rpc', 'discover', 'JSON', 'RPC']));
});

test('normalizes full-width and half-width queries consistently and needs no browser globals', () => {
  expect(tokenizeDocSearch('ＴｉｋＴｏｏｌｓ')).toContain('TikTools');
  expect(tokenizeDocSearch('ﾌﾟﾗｸﾞｲﾝ')).toEqual(tokenizeDocSearch('プラグイン'));
});
