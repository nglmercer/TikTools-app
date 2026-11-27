import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { nextTick, type App } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createSessionRecovery, sessionRecoveryKey } from '@/shared/api/session-recovery.ts';
import SessionRecoveryDialog from '../../../../../../src/web/shared/ui/dialog/SessionRecoveryDialog.vue';
import { defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

function mountDialog(backoff: readonly number[] = [60_000]) {
  const retry = vi.fn().mockResolvedValue('unavailable');
  const recovery = createSessionRecovery({ pairUrl: '/', retry, backoff });
  const wrapper = mountWithPlugins(SessionRecoveryDialog, {
    attachTo: document.body,
    global: {
      plugins: [
        {
          install(app: App): void {
            app.provide(sessionRecoveryKey, recovery);
          },
        },
      ],
    },
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
    expect(wrapper.get('h2').text()).toBe(t('sessionRecovery.expiredTitle'));
    const buttons = wrapper.findAll('footer button');
    expect(buttons.map((button) => button.text())).toEqual([t('sessionRecovery.pair'), t('sessionRecovery.retry')]);
    await defined(buttons[1]).trigger('click');
    expect(retry).toHaveBeenCalledOnce();
  });

  it('offers retry first when the host is unreachable', async () => {
    const { wrapper, recovery, retry } = mountDialog([0]);
    await exhaust(recovery);
    expect(wrapper.get('h2').text()).toBe(t('sessionRecovery.unavailableTitle'));
    const buttons = wrapper.findAll('footer button');
    expect(buttons.map((button) => button.text())).toEqual([t('sessionRecovery.retry'), t('sessionRecovery.pair')]);
    await defined(buttons[0]).trigger('click');
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
    expect(wrapper.get('h2').text()).toBe(t('sessionRecovery.expiredTitle'));
  });
});
