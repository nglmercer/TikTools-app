import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { AccountPlan } from '@/entities/cloud-account/api/account-types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudPlanUsage from './CloudPlanUsage.vue';

const t = (key: string): string => testMessage('en', key);

const freePlan: AccountPlan = {
  plan: 'free',
  limits: { maxRecords: 1000, maxBytes: 8_388_608, maxAssets: 200, maxAssetsBytes: 33_554_432, maxOverlays: 50, maxOverlayBytes: 16_777_216 },
  usage: { records: 3, recordBytes: 120, assets: 1, assetBytes: 5, overlays: 0, overlayBytes: 0 },
};

function mountWith(plan: () => unknown) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'cloud.plan.get') return plan();
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudPlanUsage, { global: { plugins: storePlugins(client) } });
  return { wrapper, calls };
}

describe('CloudPlanUsage', () => {
  it('shows the plan name and each meter against its ceiling, in bytes where needed', async () => {
    const { wrapper, calls } = mountWith(() => freePlan);
    await flushPromises();
    expect(calls.map((call) => call.method)).toContain('cloud.plan.get');
    expect(wrapper.text()).toContain(`${t('account.plan.plan')}: free`);
    expect(wrapper.text()).toContain(`3 ${t('account.plan.of')} 1000`);
    expect(wrapper.text()).toContain(`120 B ${t('account.plan.of')} 8.0 MB`);
    expect(wrapper.text()).toContain(`1 ${t('account.plan.of')} 200`);
    expect(wrapper.text()).toContain(`5 B ${t('account.plan.of')} 32.0 MB`);
    expect(wrapper.findAll('progress')).toHaveLength(6);
  });

  it('says the plan is unavailable instead of showing zero usage when the read fails', async () => {
    const { wrapper } = mountWith(() => {
      throw new ControlCallError('unavailable', 'Cloud sign-in is required');
    });
    await flushPromises();
    expect(wrapper.text()).toContain(t('account.plan.unavailable'));
    expect(wrapper.findAll('progress')).toHaveLength(0);
  });
});
