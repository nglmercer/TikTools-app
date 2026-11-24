import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseNotification from '../../../../../../src/web/shared/ui/notification/BaseNotification.vue';

describe('BaseNotification', () => {
  it('announces an error and exposes independent action and dismiss controls', async () => {
    const wrapper = mountWithPlugins(BaseNotification, {
      props: { tone: 'error', message: 'Enter a username', actionLabel: 'Connections' },
    });
    expect(wrapper.get('[role="alert"]').text()).toContain('Enter a username');
    await wrapper.get('.base-notification__action').trigger('click');
    expect(wrapper.emitted('action')).toHaveLength(1);
    await wrapper.get('.base-notification__dismiss').trigger('click');
    expect(wrapper.emitted('dismiss')).toHaveLength(1);
    expect(wrapper.emitted('action')).toHaveLength(1);
  });

  it('supports non-actionable status messages', () => {
    const wrapper = mountWithPlugins(BaseNotification, { props: { message: 'Saved' } });
    expect(wrapper.get('[role="status"]').text()).toBe('Saved');
    expect(wrapper.find('.base-notification__action').exists()).toBe(false);
  });
});
