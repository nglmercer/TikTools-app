import { afterEach, describe, expect, test } from 'bun:test';

import {
  autocompleteContributions,
  contributionFieldsForTrigger,
  isAutocompletePathAvailable,
  isAutocompletePluginAvailable,
  providerPluginIdForPath,
  registerAutocompleteContribution,
  resetAutocompleteRegistry,
  setAutocompletePluginState,
  syncAutocompletePluginStates,
  unregisterAutocompleteContribution,
  unregisterAutocompleteContributionsForPlugin,
} from './autocomplete-registry.ts';

afterEach(() => {
  resetAutocompleteRegistry();
});

describe('provider-namespace convention', () => {
  test('attributes event.intel.providers.<pluginId> paths to that plugin', () => {
    expect(providerPluginIdForPath('event.intel.providers.textintel.comment.moderation.blocked')).toBe('textintel');
    expect(providerPluginIdForPath('event.intel.providers.demo')).toBe('demo');
    expect(providerPluginIdForPath('event.intel.comment.tts.text')).toBeUndefined();
    expect(providerPluginIdForPath('event.data.comment')).toBeUndefined();
  });

  test('unknown plugins fail open, disabled or uninstalled plugins hide', () => {
    const path = 'event.intel.providers.demo.comment.normalized';
    expect(isAutocompletePathAvailable(path)).toBe(true);
    syncAutocompletePluginStates([{ id: 'demo', installed: true, enabled: false }]);
    expect(isAutocompletePathAvailable(path)).toBe(false);
    syncAutocompletePluginStates([{ id: 'demo', installed: false, enabled: false }]);
    expect(isAutocompletePathAvailable(path)).toBe(false);
    syncAutocompletePluginStates([{ id: 'demo', installed: true, enabled: true }]);
    expect(isAutocompletePathAvailable(path)).toBe(true);
  });

  test('shared intel envelopes and host-stamped processing stay core-owned', () => {
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    expect(isAutocompletePathAvailable('event.intel')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.providers')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.processing.status')).toBe(true);
    expect(isAutocompletePathAvailable('event.data.comment')).toBe(true);
  });
});

describe('textintel builtins', () => {
  test('stable views are suggested while TextIntel is available', () => {
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.user.nickname.tts.text', 'tiktok.gift')).toBe(true);
    setAutocompletePluginState('textintel', { installed: true, enabled: true });
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
  });

  test('stable views hide while TextIntel is disabled or uninstalled', () => {
    for (const state of [
      { installed: true, enabled: false },
      { installed: false, enabled: false },
    ]) {
      syncAutocompletePluginStates([{ id: 'textintel', ...state }]);
      expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(false);
      expect(isAutocompletePathAvailable('event.intel.user.nickname.tts.text', 'tiktok.gift')).toBe(false);
      // Core paths are untouched by the plugin state.
      expect(isAutocompletePathAvailable('event.data.comment', 'tiktok.chat')).toBe(true);
    }
  });

  test('moderation verdict field is pushed for chat and gated by availability', () => {
    const blocked = 'event.intel.providers.textintel.comment.moderation.blocked';
    expect(contributionFieldsForTrigger('tiktok.chat').map((field) => field.path)).toContain(blocked);
    expect(contributionFieldsForTrigger('tiktok.gift').map((field) => field.path)).not.toContain(blocked);
    // Union queries (no trigger) keep the discovery path.
    expect(contributionFieldsForTrigger().map((field) => field.path)).toContain(blocked);
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    expect(contributionFieldsForTrigger('tiktok.chat')).toEqual([]);
    expect(contributionFieldsForTrigger('tiktok.chat', { includeDisabled: true }).map((field) => field.path))
      .toContain(blocked);
  });
});

describe('register / unregister API', () => {
  test('registering an id twice replaces the contribution', () => {
    registerAutocompleteContribution({ id: 'demo.views', pluginId: 'demo', prefixes: ['event.data.demo.'] });
    registerAutocompleteContribution({ id: 'demo.views', pluginId: 'demo', prefixes: ['event.data.other.'] });
    expect(isAutocompletePathAvailable('event.data.demo.deep')).toBe(true);
    expect(autocompleteContributions().filter((entry) => entry.id === 'demo.views')).toHaveLength(1);
  });

  test('unregistering by id or by plugin restores fail-open', () => {
    registerAutocompleteContribution({ id: 'demo.views', pluginId: 'demo', prefixes: ['event.data.demo.'] });
    registerAutocompleteContribution({ id: 'demo.extra', pluginId: 'demo', paths: ['event.data.extra'] });
    syncAutocompletePluginStates([{ id: 'demo', installed: true, enabled: false }]);
    expect(isAutocompletePathAvailable('event.data.demo.deep')).toBe(false);
    expect(unregisterAutocompleteContribution('demo.views')).toBe(true);
    expect(unregisterAutocompleteContribution('demo.views')).toBe(false);
    expect(isAutocompletePathAvailable('event.data.demo.deep')).toBe(true);
    expect(unregisterAutocompleteContributionsForPlugin('demo')).toEqual(['demo.extra']);
    expect(unregisterAutocompleteContributionsForPlugin('demo')).toEqual([]);
  });

  test('shared prefixes stay visible while at least one owner is available', () => {
    registerAutocompleteContribution({ id: 'a.views', pluginId: 'a', prefixes: ['event.intel.comment.'] });
    registerAutocompleteContribution({ id: 'b.views', pluginId: 'b', prefixes: ['event.intel.comment.'] });
    syncAutocompletePluginStates([
      { id: 'textintel', installed: true, enabled: false },
      { id: 'a', installed: true, enabled: false },
      { id: 'b', installed: true, enabled: true },
    ]);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    syncAutocompletePluginStates([
      { id: 'textintel', installed: true, enabled: false },
      { id: 'a', installed: true, enabled: false },
      { id: 'b', installed: true, enabled: false },
    ]);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(false);
  });

  test('trigger scoping gates only the declared triggers', () => {
    registerAutocompleteContribution({
      id: 'demo.chat',
      pluginId: 'demo',
      prefixes: ['event.data.demo.'],
      triggers: ['tiktok.chat'],
    });
    syncAutocompletePluginStates([{ id: 'demo', installed: true, enabled: false }]);
    expect(isAutocompletePathAvailable('event.data.demo.deep', 'tiktok.chat')).toBe(false);
    expect(isAutocompletePathAvailable('event.data.demo.deep', 'tiktok.gift')).toBe(true);
    // Union queries cannot prove the path belongs to another trigger.
    expect(isAutocompletePathAvailable('event.data.demo.deep')).toBe(false);
  });

  test('registration validates ownership', () => {
    expect(() => registerAutocompleteContribution({ id: '', pluginId: 'demo' })).toThrow();
    expect(() => registerAutocompleteContribution({ id: 'x', pluginId: '  ' })).toThrow();
  });
});

describe('plugin state sync', () => {
  test('sync is authoritative: absent plugins return to fail-open', () => {
    syncAutocompletePluginStates([{ id: 'demo', installed: true, enabled: false }]);
    expect(isAutocompletePluginAvailable('demo')).toBe(false);
    syncAutocompletePluginStates([]);
    expect(isAutocompletePluginAvailable('demo')).toBe(true);
  });

  test('reset restores builtins and drops custom state', () => {
    registerAutocompleteContribution({ id: 'demo.views', pluginId: 'demo', prefixes: ['event.data.demo.'] });
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    resetAutocompleteRegistry();
    expect(isAutocompletePluginAvailable('textintel')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    expect(autocompleteContributions().map((entry) => entry.id).sort()).toEqual([
      'textintel.moderation',
      'textintel.stable-views',
    ]);
  });
});
