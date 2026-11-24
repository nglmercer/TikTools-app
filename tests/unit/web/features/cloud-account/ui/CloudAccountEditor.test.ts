const t = (key: string): string => testMessage('en', key);
import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import type { AccountStatus } from '@/entities/cloud-account/api/account-types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudAccountEditor from '../../../../../../src/web/features/cloud-account/ui/CloudAccountEditor.vue';

function signedIn(overrides: Partial<AccountStatus> = {}): AccountStatus {
  return {
    origin: 'http://127.0.0.1:3000',
    state: 'signed-in',
    accountId: 'user-1',
    email: 'user@example.com',
    displayName: 'Test User',
    persistence: 'keychain',
    error: null,
    ...overrides,
  };
}

function setup(status: AccountStatus) {
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloud.status') return status;
    if (method === 'cloud.restore') {
      throw new ControlCallError('unavailable', 'no saved cloud session', { reason: 'no-saved-session' });
    }
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudAccountEditor, {
    global: { plugins: storePlugins(client) },
  });
  return wrapper;
}

describe('CloudAccountEditor', () => {
  afterEach(async () => { await activateLocale('en'); });
  it('shows only a sign-in button when signed out, with no technical details', async () => {
    const wrapper = setup({ origin: 'https://cloud.example', state: 'signed-out', persistence: 'keychain' });
    await flushPromises();
    expect(wrapper.text()).toContain(t('account.signIn'));
    expect(wrapper.text()).not.toContain('cloud.example');
    expect(wrapper.text()).not.toContain('TIKTOOLS_CLOUD_ORIGIN');
  });

  it('turns network failures into a friendly message', async () => {
    const wrapper = setup({
      origin: 'https://cloud.example',
      state: 'signed-out',
      persistence: 'keychain',
      error: 'cloud login failed: error sending request for url',
    });
    await flushPromises();
    expect(wrapper.text()).toContain("Couldn't reach TikTools Cloud");
    expect(wrapper.text()).not.toContain('error sending request');
  });

  it('shows the email under the display name and hides the origin', async () => {
    const wrapper = setup(signedIn());
    await flushPromises();
    expect(wrapper.text()).toContain('user@example.com');
    expect(wrapper.text()).toContain(t('account.signedIn'));
    expect(wrapper.text()).not.toContain('127.0.0.1');
  });

  it('prefers the verified display name over the raw account id', async () => {
    const wrapper = setup(signedIn());
    await flushPromises();
    expect(wrapper.text()).toContain('Test User');
    expect(wrapper.text()).not.toContain('user-1');
  });

  it('falls back to email and then account id', async () => {
    const byEmail = setup(signedIn({ displayName: null }));
    await flushPromises();
    expect(byEmail.text()).toContain('user@example.com');

    const byId = setup(signedIn({ displayName: null, email: null }));
    await flushPromises();
    expect(byId.text()).toContain('user-1');
  });

  it('shows manual login and cancellation', async () => {
    const [signIn, cancel, manual] = [t('account.signIn'), t('common.cancel'), t('account.openManually')] as const;
    let finish!: (status: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { finish = resolve; });
    const out: AccountStatus = { origin: 'https://cloud.example', state: 'signed-out', persistence: 'keychain' };
    const { client } = createFakeControlClient((method) => {
      if (method === 'cloud.status') return out;
      if (method === 'cloud.restore') throw new ControlCallError('unavailable', 'no saved session', { reason: 'no-saved-session' });
      if (method === 'cloud.login.reserve') return { operationId: '0123456789abcdef0123456789abcdef' };
      if (method === 'cloud.login.begin') return { browserUrl: 'https://cloud.example/desktop/login' };
      if (method === 'cloud.login.open') throw new Error('browser unavailable');
      if (method === 'cloud.login.complete') return pending;
      if (method === 'cloud.login.cancel') return out;
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudAccountEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.findAll('button').find((button) => button.text() === signIn)!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain(manual);
    expect(wrapper.text()).toContain('https://cloud.example/desktop/login');
    expect(wrapper.get('.cloud-account__waiting').text()).toContain(t('account.waiting'));
    expect(wrapper.find('.cloud-account__spin').exists()).toBe(true);
    expect(wrapper.get('.cloud-account').attributes('aria-busy')).toBe('true');
    await wrapper.findAll('button').find((button) => button.text() === cancel)!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain(signIn);
    expect(wrapper.text()).not.toContain('https://cloud.example/desktop/login');
    expect(wrapper.find('.cloud-account__spin').exists()).toBe(false);
    expect(wrapper.get('.cloud-account').attributes('aria-busy')).toBeUndefined();
    finish(out);
    await flushPromises();
    wrapper.unmount();
  });

  it('lays out the summary and plan beside the backups, with an icon-only sign out', async () => {
    const calls: string[] = [];
    const { client } = createFakeControlClient((method) => {
      calls.push(method);
      if (method === 'cloud.status') return signedIn();
      if (method === 'cloud.plan.get') {
        return {
          plan: 'free',
          limits: { maxRecords: 1000, maxBytes: 8_388_608, maxAssets: 200, maxAssetsBytes: 33_554_432, maxOverlays: 50, maxOverlayBytes: 16_777_216, maxBackups: 3, maxBackupBytes: 16_777_216 },
          usage: { records: 0, recordBytes: 0, assets: 0, assetBytes: 0, overlays: 0, overlayBytes: 0, backups: 0, backupBytes: 0 },
        };
      }
      if (method === 'cloud.backup.list') return { items: [] };
      if (method === 'cloud.logout') return { origin: null, state: 'signed-out', persistence: 'keychain' };
      throw new Error(`unexpected ${method}`);
    });
    const wrapper = mountWithPlugins(CloudAccountEditor, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    const aside = wrapper.get('.cloud-workspace__aside');
    expect(aside.find('.account-summary').exists()).toBe(true);
    expect(aside.find('.cloud-plan').exists()).toBe(true);
    expect(aside.get('.account-summary__avatar').text()).toBe('TU');
    expect(aside.get('.account-summary__plan').text()).toBe('Free');
    expect(wrapper.get('.cloud-workspace__main').find('.backup-section').exists()).toBe(true);
    // The old framed "Account" group is gone once signed in.
    expect(wrapper.find('fieldset').exists()).toBe(false);
    expect(wrapper.find('[data-guide-scope="account"]').exists()).toBe(true);
    await aside.get(`button[aria-label="${t('account.signOut')}"]`).trigger('click');
    await flushPromises();
    expect(calls).toContain('cloud.logout');
    wrapper.unmount();
  });

});
