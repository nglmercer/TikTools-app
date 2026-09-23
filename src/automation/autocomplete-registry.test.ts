import { afterEach, describe, expect, test } from 'bun:test';

import {
  applySnapshotContributions,
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
import type { PluginAutocompleteContribution } from './behavior/types.ts';

afterEach(() => {
  resetAutocompleteRegistry();
});

/** Host-stamped shape of the TextIntel manifest `autocomplete` section. */
function textintelSnapshot(): PluginAutocompleteContribution[] {
  const source = { kind: 'plugin', pluginId: 'textintel' } as const;
  return [
    {
      id: 'textintel/stable-views',
      pluginId: 'textintel',
      prefixes: ['event.intel.comment.', 'event.intel.user.'],
      source: { ...source },
    },
    {
      id: 'textintel/moderation',
      pluginId: 'textintel',
      fields: [
        {
          path: 'event.intel.providers.textintel.comment.moderation.blocked',
          kind: 'boolean',
          label: { default: 'Moderation blocked', i18key: '' },
          hint: { default: 'True when TextIntel blocked this chat message.', i18key: '' },
        },
      ],
      triggers: ['tiktok.chat'],
      source: { ...source },
    },
  ];
}

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

describe('snapshot-seeded contributions', () => {
  test('registry starts empty: unowned paths fail open', () => {
    expect(autocompleteContributions()).toEqual([]);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    expect(contributionFieldsForTrigger('tiktok.chat')).toEqual([]);
  });

  test('seeded stable views are suggested while their owner is available', () => {
    applySnapshotContributions(textintelSnapshot());
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.user.nickname.tts.text', 'tiktok.gift')).toBe(true);
    setAutocompletePluginState('textintel', { installed: true, enabled: true });
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
  });

  test('seeded views hide while their owner is disabled, uninstalled, or unavailable', () => {
    applySnapshotContributions(textintelSnapshot());
    for (const state of [
      { installed: true, enabled: false },
      { installed: false, enabled: false },
      { installed: true, enabled: true, available: false },
    ]) {
      syncAutocompletePluginStates([{ id: 'textintel', ...state }]);
      expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(false);
      expect(isAutocompletePathAvailable('event.intel.user.nickname.tts.text', 'tiktok.gift')).toBe(false);
      // Core paths are untouched by the plugin state.
      expect(isAutocompletePathAvailable('event.data.comment', 'tiktok.chat')).toBe(true);
    }
  });

  test('moderation verdict field is pushed for chat and gated by availability', () => {
    applySnapshotContributions(textintelSnapshot());
    const blocked = 'event.intel.providers.textintel.comment.moderation.blocked';
    const chat = contributionFieldsForTrigger('tiktok.chat');
    expect(chat.map((field) => field.path)).toContain(blocked);
    expect(chat.find((field) => field.path === blocked)?.label.en).toBe('Moderation blocked');
    expect(contributionFieldsForTrigger('tiktok.gift').map((field) => field.path)).not.toContain(blocked);
    // Union queries (no trigger) keep the discovery path.
    expect(contributionFieldsForTrigger().map((field) => field.path)).toContain(blocked);
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    expect(contributionFieldsForTrigger('tiktok.chat')).toEqual([]);
    expect(contributionFieldsForTrigger('tiktok.chat', { includeDisabled: true }).map((field) => field.path))
      .toContain(blocked);
  });

  test('re-applying the snapshot replaces seeded entries but keeps local pushes', () => {
    registerAutocompleteContribution({ id: 'local.views', pluginId: 'local', prefixes: ['event.data.local.'] });
    applySnapshotContributions(textintelSnapshot());
    expect(autocompleteContributions().map((entry) => entry.id).sort()).toEqual([
      'local.views',
      'textintel/moderation',
      'textintel/stable-views',
    ]);
    // The host drops contributions on disable: an empty snapshot clears the
    // seeded origin while the local push survives.
    applySnapshotContributions([]);
    expect(autocompleteContributions().map((entry) => entry.id)).toEqual(['local.views']);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
  });

  test('malformed snapshot entries are skipped, never fatal', () => {
    applySnapshotContributions([
      ...textintelSnapshot(),
      { id: '  ', pluginId: 'demo' } as unknown as PluginAutocompleteContribution,
      { id: 'demo/empty', pluginId: '  ' } as unknown as PluginAutocompleteContribution,
    ]);
    expect(autocompleteContributions().map((entry) => entry.id).sort()).toEqual([
      'textintel/moderation',
      'textintel/stable-views',
    ]);
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
      { id: 'a', installed: true, enabled: false },
      { id: 'b', installed: true, enabled: true },
    ]);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
    syncAutocompletePluginStates([
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

  test('reset drops every origin and returns to fail-open', () => {
    registerAutocompleteContribution({ id: 'demo.views', pluginId: 'demo', prefixes: ['event.data.demo.'] });
    applySnapshotContributions(textintelSnapshot());
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    resetAutocompleteRegistry();
    expect(autocompleteContributions()).toEqual([]);
    expect(isAutocompletePluginAvailable('textintel')).toBe(true);
    expect(isAutocompletePathAvailable('event.intel.comment.tts.text', 'tiktok.chat')).toBe(true);
  });
});
