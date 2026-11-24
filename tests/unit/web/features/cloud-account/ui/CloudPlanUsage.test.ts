import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { AccountPlan } from '@/entities/cloud-account/api/account-types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudPlanUsage from '../../../../../../src/web/features/cloud-account/ui/CloudPlanUsage.vue';

const t = (key: string): string => testMessage('en', key);

const freePlan: AccountPlan = {
  plan: 'free',
  limits: { maxRecords: 1000, maxBytes: 8_388_608, maxAssets: 200, maxAssetsBytes: 33_554_432, maxOverlays: 50, maxOverlayBytes: 16_777_216, maxBackups: 3, maxBackupBytes: 16_777_216 },
  usage: { records: 3, recordBytes: 120, assets: 1, assetBytes: 5, overlays: 0, overlayBytes: 0, backups: 0, backupBytes: 0 },
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
  it('lists four areas with one count, one bar and the space used each', async () => {
    const { wrapper, calls } = mountWith(() => freePlan);
    await flushPromises();
    expect(calls.map((call) => call.method)).toContain('cloud.plan.get');
    expect(wrapper.findAll('.cloud-plan__label').map((node) => node.text())).toEqual([
      t('account.plan.workspace'),
      t('account.plan.assets'),
      t('account.plan.overlays'),
      t('account.plan.backups'),
    ]);
    expect(wrapper.text()).toContain(`3 ${t('account.plan.of')} 1,000`);
    expect(wrapper.text()).toContain(`120 B ${t('account.plan.of')} 8.0 MB`);
    expect(wrapper.text()).toContain(`1 ${t('account.plan.of')} 200`);
    expect(wrapper.text()).toContain(`5 B ${t('account.plan.of')} 32.0 MB`);
    expect(wrapper.text()).toContain(`0 ${t('account.plan.of')} 3`);
    // One bar per area, filled by whichever of count or space is closer to its limit.
    const bars = wrapper.findAll('progress');
    expect(bars).toHaveLength(4);
    expect(Number(bars[0]!.attributes('value'))).toBeCloseTo(3 / 1000);
  });

  it('folds to its tightest area on a phone and opens on demand', async () => {
    const original = window.matchMedia;
    window.matchMedia = ((query: string) => ({
      matches: query.includes('600px'), media: query, addEventListener: () => {}, removeEventListener: () => {},
      addListener: () => {}, removeListener: () => {}, onchange: null, dispatchEvent: () => false,
    })) as unknown as typeof window.matchMedia;
    try {
      const { wrapper } = mountWith(() => ({ ...freePlan, usage: { ...freePlan.usage, backups: 2, backupBytes: 100 } }));
      await flushPromises();
      expect(wrapper.findAll('.cloud-plan__area')).toHaveLength(1);
      expect(wrapper.text()).toContain(t('account.plan.backups'));
      await wrapper.get('button').trigger('click');
      expect(wrapper.findAll('.cloud-plan__area')).toHaveLength(4);
      expect(wrapper.get('button').attributes('aria-expanded')).toBe('true');
    } finally {
      window.matchMedia = original;
    }
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
