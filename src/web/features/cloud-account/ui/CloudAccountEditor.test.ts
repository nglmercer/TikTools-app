import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { AccountStatus } from '@/entities/cloud-account/api/account-types.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudAccountEditor from './CloudAccountEditor.vue';

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
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudAccountEditor, {
    global: { plugins: storePlugins(client) },
  });
  return wrapper;
}

describe('CloudAccountEditor', () => {
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
});
