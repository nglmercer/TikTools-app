import { describe, expect, it } from 'vitest';
import { legacySettingsRedirect } from './legacy-settings.ts';

describe('legacySettingsRedirect', () => {
  it('sends runtime variables to Automations and plugin updates to Plugins', () => {
    expect(legacySettingsRedirect({ section: 'globals' })).toEqual({ path: '/automations', query: { view: 'variables' } });
    expect(legacySettingsRedirect({ section: 'plugins' })).toEqual({ path: '/plugins', query: { view: 'preferences' } });
    expect(legacySettingsRedirect({ section: 'advanced' })).toEqual({ path: '/plugins', query: { view: 'preferences' } });
  });

  it('lets the sections Settings still owns through', () => {
    for (const section of ['general', 'account', 'collaboration', 'updates', 'unknown', undefined]) {
      expect(legacySettingsRedirect(section ? { section } : {})).toBe(true);
    }
  });
});
