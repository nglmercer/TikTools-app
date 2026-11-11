import { describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import AppStatusBar from './AppStatusBar.vue';

function mountStatusBar(props: Record<string, unknown>) {
  return mountWithPlugins(AppStatusBar, { props: { creator: '', viewers: 0, ...props } as never });
}

describe('AppStatusBar', () => {
  it('shows the live creator, viewers and disconnect controls', async () => {
    const wrapper = mountStatusBar({ status: 'connected', creator: '@luna', viewers: 1520 });
    expect(wrapper.get('[role="status"]').text()).toBe('Live');
    expect(wrapper.text()).toContain('@luna');
    expect(wrapper.text()).toContain('1,520 viewers');
    const buttons = wrapper.findAll('button');
    expect(buttons.map((button) => button.text())).toEqual(['Reconnect', 'Disconnect']);
    await buttons[1]!.trigger('click');
    expect(wrapper.emitted('disconnect')).toHaveLength(1);
  });

  it('offers connect while offline and hides viewers', async () => {
    const wrapper = mountStatusBar({ status: 'disconnected' });
    expect(wrapper.text()).not.toContain('viewers');
    await wrapper.get('button').trigger('click');
    expect(wrapper.emitted('connect')).toHaveLength(1);
  });

  it('reports retry progress and allows cancellation while busy', async () => {
    const wrapper = mountStatusBar({ status: 'retrying', reconnectAttempt: 2, maxReconnectAttempts: 5 });
    expect(wrapper.get('[role="status"]').text()).toBe('Reconnecting 2/5…');
    expect(wrapper.get('button').text()).toBe('Cancel');
    expect(wrapper.get('button').attributes('disabled')).toBeUndefined();
    await wrapper.get('button').trigger('click');
    expect(wrapper.emitted('disconnect')).toHaveLength(1);
  });

  it('turns an error into a reconnect action', async () => {
    const wrapper = mountStatusBar({ status: 'error' });
    await wrapper.get('button').trigger('click');
    expect(wrapper.emitted('reconnect')).toHaveLength(1);
  });

  it('does not repeat the connection on the page that already shows it', () => {
    const wrapper = mountStatusBar({ status: 'connected', creator: '@luna', viewers: 3, hideConnection: true });
    expect(wrapper.find('[role="status"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('@luna');
    expect(wrapper.findAll('button')).toHaveLength(0);
  });
});
