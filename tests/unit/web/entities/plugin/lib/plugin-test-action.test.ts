import { describe, expect, it } from 'vitest';
import { findTestConnectionAction } from '../../../../../../src/web/entities/plugin/lib/plugin-test-action.ts';
import { actionType } from '../../../../support/automation.ts';

const actionTypes = [
  actionType({ id: 'l4d2.test_connection', source: { kind: 'plugin', pluginId: 'l4d2.interactive' } }),
  actionType({ id: 'l4d2.spawn_common', source: { kind: 'plugin', pluginId: 'l4d2.interactive' } }),
  actionType({ id: 'other.test_connection', source: { kind: 'plugin', pluginId: 'other.plugin' } }),
  actionType({ id: 'builtin.test_connection', source: { kind: 'builtin' } }),
];

describe('findTestConnectionAction', () => {
  it('finds the plugin self-check action', () => {
    expect(findTestConnectionAction(actionTypes, 'l4d2.interactive')).toBe('l4d2.test_connection');
  });

  it('ignores other plugins and builtin actions with the same suffix', () => {
    expect(findTestConnectionAction(actionTypes, 'other.plugin')).toBe('other.test_connection');
    expect(findTestConnectionAction(actionTypes, 'missing.plugin')).toBeNull();
    expect(findTestConnectionAction([], 'l4d2.interactive')).toBeNull();
  });
});
