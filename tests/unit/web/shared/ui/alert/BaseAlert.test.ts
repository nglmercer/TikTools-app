import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseAlert from '../../../../../../src/web/shared/ui/alert/BaseAlert.vue';

describe('BaseAlert', () => {
  it('announces errors assertively and renders the title', () => {
    const wrapper = mountWithPlugins(BaseAlert, { props: { title: 'Something failed' } });
    const alert = wrapper.get('[role="alert"]');
    expect(alert.classes()).toContain('base-alert--error');
    expect(alert.get('.base-alert__title').text()).toBe('Something failed');
  });

  it('reports other tones as plain status', () => {
    for (const tone of ['warning', 'info', 'ok'] as const) {
      const wrapper = mountWithPlugins(BaseAlert, { props: { tone, title: `${tone} note` } });
      const alert = wrapper.get('[role="status"]');
      expect(alert.classes()).toContain(`base-alert--${tone}`);
    }
  });

  it('supports a compact single-idea layout', () => {
    const wrapper = mountWithPlugins(BaseAlert, { props: { title: 'Short note', compact: true } });
    expect(wrapper.get('.base-alert').classes()).toContain('base-alert--compact');
  });

  it('renders follow-up content in the body', () => {
    const wrapper = mountWithPlugins(BaseAlert, {
      props: { tone: 'info', title: 'Heads up' },
      slots: { default: '<ul><li>first</li><li>second</li></ul>' },
    });
    expect(wrapper.get('.base-alert__body').text()).toContain('first');
  });
});
