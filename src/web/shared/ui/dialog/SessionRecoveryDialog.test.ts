import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createSessionRecovery, sessionRecoveryKey } from '@/shared/api/session-recovery.ts';
import SessionRecoveryDialog from './SessionRecoveryDialog.vue';

enableAutoUnmount(afterEach);

function mountDialog(backoff: readonly number[] = [60_000]) {
  const retry = vi.fn().mockResolvedValue('unavailable');
  const recovery = createSessionRecovery({ pairUrl: '/', retry, backoff });
  const wrapper = mountWithPlugins(SessionRecoveryDialog, {
    attachTo: document.body,
    global: { provide: { [sessionRecoveryKey as symbol]: recovery } },
  });
  return { wrapper, recovery, retry };
}

async function exhaust(recovery: ReturnType<typeof createSessionRecovery>): Promise<void> {
  recovery.report('unavailable');
  await vi.waitFor(() => expect(recovery.state.value).toBe('unavailable'));
  await nextTick();
}

describe('SessionRecoveryDialog', () => {
  it('stays closed while the session is healthy', async () => {
    const { wrapper } = mountDialog();
    await nextTick();
    expect(wrapper.find('dialog').element.hasAttribute('open')).toBe(false);
  });

  it('offers a fresh pairing when the cookie was rejected', async () => {
    const { wrapper, recovery, retry } = mountDialog();
    recovery.report('expired');
    await nextTick();
    expect(wrapper.get('h2').text()).toBe('Session expired');
    const buttons = wrapper.findAll('footer button');
    expect(buttons.map((button) => button.text())).toEqual(['Pair again', 'Retry now']);
    await buttons[1]!.trigger('click');
    expect(retry).toHaveBeenCalledOnce();
  });

  it('offers retry first when the host is unreachable', async () => {
    const { wrapper, recovery, retry } = mountDialog([0]);
    await exhaust(recovery);
    expect(wrapper.get('h2').text()).toBe('Host unavailable');
    const buttons = wrapper.findAll('footer button');
    expect(buttons.map((button) => button.text())).toEqual(['Retry now', 'Pair again']);
    await buttons[0]!.trigger('click');
    expect(retry).toHaveBeenCalled();
  });

  it('dismisses without pretending the session recovered', async () => {
    const { wrapper, recovery } = mountDialog([0]);
    await exhaust(recovery);
    await wrapper.get('button[aria-label="Close"]').trigger('click');
    await nextTick();
    expect(wrapper.find('dialog').element.hasAttribute('open')).toBe(false);
    expect(recovery.state.value).toBe('unavailable');
    // A new failure is a new event: the dialog comes back.
    recovery.report('expired');
    await nextTick();
    expect(wrapper.get('h2').text()).toBe('Session expired');
  });
});
