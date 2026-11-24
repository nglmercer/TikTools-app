import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { ConnectionStatus } from '@/types.ts';
import ConnectionControls from '../../../../../../src/web/features/connection-controls/ui/ConnectionControls.vue';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

describe('ConnectionControls', () => {
  it.each<ConnectionStatus>(['connecting', 'retrying'])('offers cancellation while %s', async (status) => {
    const wrapper = mountWithPlugins(ConnectionControls, { props: { status } });
    const cancel = wrapper.get('button');
    expect(cancel.text()).toBe(t('common.cancel'));
    expect(cancel.attributes('disabled')).toBeUndefined();
    await cancel.trigger('click');
    expect(wrapper.emitted('disconnect')).toHaveLength(1);
    expect(wrapper.emitted('connect')).toBeUndefined();
    expect(wrapper.emitted('reconnect')).toBeUndefined();
  });
});
