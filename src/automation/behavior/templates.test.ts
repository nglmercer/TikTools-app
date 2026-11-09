import { describe, expect, test } from 'bun:test';

import type { AutomationEvent } from '../types.ts';
import { readEventPath, renderJsonTemplate, renderTemplate } from './templates.ts';

function chat(comment: string): AutomationEvent {
  return {
    type: 'chat',
    data: { comment, count: 3 },
    user: { uniqueId: 'viewer' },
  } as unknown as AutomationEvent;
}

describe('renderTemplate', () => {
  test('substitutes event paths and blanks missing ones', () => {
    expect(renderTemplate('{{user.uniqueId}}: {{data.comment}} {{data.missing}}', chat('hi'))).toBe('viewer: hi ');
  });

  test('does not resolve prototype members', () => {
    expect(readEventPath(chat('hi'), 'data.constructor')).toBeUndefined();
    expect(readEventPath(chat('hi'), 'data.__proto__')).toBeUndefined();
  });
});

describe('renderJsonTemplate', () => {
  test('escapes values inside string literals', () => {
    const value = renderJsonTemplate('{"text": "{{data.comment}}", "n": {{data.count}}}', chat('say "hi" \\ now\n'));
    expect(value).toEqual({ text: 'say "hi" \\ now\n', n: 3 });
  });

  test('renders once, so placeholders inside viewer text stay literal', () => {
    const value = renderJsonTemplate('{"text": "{{data.comment}}"}', chat('{{user.uniqueId}}'));
    expect(value).toEqual({ text: '{{user.uniqueId}}' });
  });

  test('tracks escaped quotes in the literal template', () => {
    const value = renderJsonTemplate('{"text": "a \\" {{data.comment}}"}', chat('"b"'));
    expect(value).toEqual({ text: 'a " "b"' });
  });

  test('rejects output that is not JSON', () => {
    expect(() => renderJsonTemplate('{"n": {{data.comment}}}', chat('oops'))).toThrow('Template did not produce valid JSON.');
  });
});
