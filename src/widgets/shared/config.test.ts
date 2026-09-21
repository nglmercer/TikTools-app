import { describe, expect, test } from 'bun:test';

import {
  DEFAULT_GATEWAY_HOST,
  DEFAULT_GATEWAY_PORT,
  parseDemoMode,
  parseFollowSettings,
  parseGiftSettings,
} from './config.ts';
import { CredentialProvider } from './credentials.ts';

describe('credential provider', () => {
  test('reads token, host, and port from the URL fragment', () => {
    const provider = new CredentialProvider({ hash: '#token=ttw_abc&host=127.0.0.1&port=18000' });
    expect(provider.token).toBe('ttw_abc');
    expect(provider.host).toBe('127.0.0.1');
    expect(provider.port).toBe(18000);
  });

  test('falls back to defaults for missing or invalid values', () => {
    const provider = new CredentialProvider({ hash: '#port=banana' });
    expect(provider.token).toBeNull();
    expect(provider.host).toBe(DEFAULT_GATEWAY_HOST);
    expect(provider.port).toBe(DEFAULT_GATEWAY_PORT);
    expect(new CredentialProvider({ hash: '#port=99999' }).port).toBe(DEFAULT_GATEWAY_PORT);
  });

  test('reads credentials only from the fragment, never from storage', () => {
    // No storage or injection inputs exist: an empty fragment means no
    // credential, even if a previous page load saw one. OBS persists the
    // Browser Source URL itself.
    expect(new CredentialProvider({}).token).toBeNull();
    expect(new CredentialProvider({ hash: '' }).token).toBeNull();
    const provider = new CredentialProvider({ hash: '#token=ttw_fragment' });
    expect(provider.token).toBe('ttw_fragment');
    expect('rememberFragmentToken' in provider).toBe(false);
  });
});

describe('demo mode', () => {
  test('parses the demo fragment param', () => {
    expect(parseDemoMode('#demo=follow')).toBe('follow');
    expect(parseDemoMode('#token=ttk_x&demo=combo')).toBe('combo');
    expect(parseDemoMode('#demo=gift')).toBe('gift');
    expect(parseDemoMode('')).toBeNull();
    expect(parseDemoMode('#demo=bogus')).toBeNull();
    expect(parseDemoMode('#token=ttk_x')).toBeNull();
  });
});

describe('widget display settings', () => {
  test('follow settings default and parse query overrides', () => {
    expect(parseFollowSettings('')).toEqual({ visibleMs: 4000, enterMs: 400, exitMs: 400, showUniqueId: true });
    expect(parseFollowSettings('?duration=2000&handle=0')).toEqual({
      visibleMs: 2000,
      enterMs: 400,
      exitMs: 400,
      showUniqueId: false,
    });
    expect(parseFollowSettings('?duration=nope&enter=-5')).toEqual({
      visibleMs: 4000,
      enterMs: 400,
      exitMs: 400,
      showUniqueId: true,
    });
  });

  test('gift settings default and parse query overrides', () => {
    expect(parseGiftSettings('')).toEqual({
      visibleMs: 5000,
      comboTimeoutMs: 3000,
      minimumDiamonds: 0,
      showImage: true,
      showDiamonds: true,
      showCount: true,
    });
    expect(parseGiftSettings('?minDiamonds=100&image=false&comboTimeout=1500')).toEqual({
      visibleMs: 5000,
      comboTimeoutMs: 1500,
      minimumDiamonds: 100,
      showImage: false,
      showDiamonds: true,
      showCount: true,
    });
  });
});
