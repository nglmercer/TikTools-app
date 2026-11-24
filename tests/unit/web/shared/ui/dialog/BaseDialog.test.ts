import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseDialog from '../../../../../../src/web/shared/ui/dialog/BaseDialog.vue';

enableAutoUnmount(afterEach);

function mountDialog(props: Record<string, unknown> = {}) {
  const wrapper = mountWithPlugins(BaseDialog, {
    attachTo: document.body,
    props: {
      title: 'Properties',
      open: false,
      'onUpdate:open': (value: boolean) => wrapper.setProps({ open: value }),
      ...props,
    },
    slots: {
      default: '<p class="content">Body</p>',
      'footer-start': '<button class="defaults">Defaults</button>',
      footer: '<button class="ok">OK</button>',
    },
  });
  return wrapper;
}

describe('BaseDialog', () => {
  it('opens modally and renders its content only while open', async () => {
    const wrapper = mountDialog();
    const dialog = wrapper.get('dialog').element as HTMLDialogElement;
    expect(dialog.open).toBe(false);
    expect(wrapper.find('.content').exists()).toBe(false);
    await wrapper.setProps({ open: true });
    await nextTick();
    expect(dialog.open).toBe(true);
    expect(wrapper.get('h2').text()).toBe('Properties');
    expect(dialog.getAttribute('aria-labelledby')).toBe(wrapper.get('h2').attributes('id'));
    expect(wrapper.find('.defaults').exists()).toBe(true);
    expect(wrapper.find('.ok').exists()).toBe(true);
  });

  it('syncs the model when the native dialog closes', async () => {
    const wrapper = mountDialog({ open: true });
    await nextTick();
    const dialog = wrapper.get('dialog').element as HTMLDialogElement;
    dialog.close();
    dialog.dispatchEvent(new Event('close'));
    await nextTick();
    expect(wrapper.props('open')).toBe(false);
  });

  it('closes from the title bar button', async () => {
    const wrapper = mountDialog({ open: true });
    await nextTick();
    await wrapper.get('button[aria-label="Close"]').trigger('click');
    expect(wrapper.props('open')).toBe(false);
  });

  it('blocks Escape when not closable', async () => {
    const wrapper = mountDialog({ open: true, closable: false });
    await nextTick();
    const cancel = new Event('cancel', { cancelable: true });
    wrapper.get('dialog').element.dispatchEvent(cancel);
    expect(cancel.defaultPrevented).toBe(true);
    expect(wrapper.find('button[aria-label="Close"]').exists()).toBe(false);
  });
});
