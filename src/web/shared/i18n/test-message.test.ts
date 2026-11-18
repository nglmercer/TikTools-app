import { expect, test } from 'vitest';
import { testMessage } from './test-message.ts';

test('resolves the same key in every locale from the real catalogs', () => {
  // Self-test of the catalog resolver: expected values must be literal.
  expect(testMessage('en', 'automations.conflict.base')).toBe('Starting version');
  expect(testMessage('es', 'automations.conflict.base')).toBe('Versión inicial');
  expect(testMessage('fr', 'automations.conflict.base')).toBeTruthy();
});

test('interpolates parameters without hardcoding translated text', () => {
  expect(testMessage('en', 'jsonView.reviewTable.revision', { revision: '7' })).toBeTruthy();
  expect(testMessage('en', 'libraryRecovery.reviewLead', { events: 2, actions: 1 })).toContain('2');
});

test('throws on an unknown key so tests cannot silently pass with a wrong string', () => {
  expect(() => testMessage('en', 'definitely.not.a.key')).toThrow('Missing i18n key');
});
