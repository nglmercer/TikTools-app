import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { defineComponent, h, ref } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import BaseImportDialog from './BaseImportDialog.vue';
import type { ImportTab, StagedImportItem } from './import-types.ts';

enableAutoUnmount(afterEach);

const tabs: ImportTab[] = [
  { value: 'local', label: 'Local' },
  { value: 'cloud', label: 'Cloud' },
  { value: 'manual', label: 'Manual' },
];

interface Options {
  tab?: string;
  staged?: StagedImportItem[];
  busy?: boolean;
  errors?: string[];
  success?: string;
  maxBytes?: number;
  tabs?: ImportTab[];
  hideTabs?: boolean;
  slots?: Record<string, string>;
}

function mountDialog(options: Options = {}) {
  const wrapper = mountWithPlugins(BaseImportDialog, {
    props: {
      open: true,
      tab: options.tab ?? 'manual',
      title: 'Bring things in',
      tabs: options.tabs ?? tabs,
      staged: options.staged ?? [],
      busy: options.busy ?? false,
      errors: options.errors ?? [],
      success: options.success ?? '',
      maxBytes: options.maxBytes,
      hideTabs: options.hideTabs ?? false,
      'onUpdate:tab': (value: string) => wrapper.setProps({ tab: value }),
    },
    slots: options.slots ?? { local: '<p class="local-body">Local list</p>', cloud: '<p class="cloud-body">Cloud list</p>' },
    attachTo: document.body,
  });
  return wrapper;
}

const staged: StagedImportItem[] = [
  { key: 'a', label: 'First', detail: '3 rules' },
  { key: 'b', label: 'Second' },
];

describe('BaseImportDialog', () => {
  it('draws a tab per source with tab semantics and shows only the selected body', async () => {
    const wrapper = mountDialog({ tab: 'local' });
    const list = wrapper.get('[role="tablist"]');
    expect(list.attributes('aria-label')).toBe('Import sources');
    const found = wrapper.findAll('[role="tab"]');
    expect(found.map((tab) => tab.text())).toEqual(['Local', 'Cloud', 'Manual']);
    expect(found.map((tab) => tab.attributes('aria-selected'))).toEqual(['true', 'false', 'false']);
    const panel = wrapper.get('[role="tabpanel"]');
    expect(panel.attributes('aria-labelledby')).toBe(found[0]!.attributes('id'));
    expect(panel.find('.local-body').exists()).toBe(true);
    expect(wrapper.find('.cloud-body').exists()).toBe(false);
    expect(wrapper.findComponent(BaseFileDrop).exists()).toBe(false);
    await list.trigger('keydown', { key: 'ArrowRight' });
    expect(wrapper.props('tab')).toBe('cloud');
    expect(wrapper.find('.cloud-body').exists()).toBe(true);
    await list.trigger('keydown', { key: 'End' });
    expect(wrapper.props('tab')).toBe('manual');
    expect(wrapper.findComponent(BaseFileDrop).exists()).toBe(true);
    expect(wrapper.get('[role="tabpanel"]').find('.local-body').exists()).toBe(false);
  });

  it('shows the drop zone as the body of the manual tab and forwards what it reads', async () => {
    const wrapper = mountDialog();
    const drop = wrapper.getComponent(BaseFileDrop);
    drop.vm.$emit('content', '{"a":1}', 'a.json');
    drop.vm.$emit('error', 'Could not read');
    expect(wrapper.emitted('content')).toEqual([['{"a":1}', 'a.json']]);
    expect(wrapper.emitted('error')).toEqual([['Could not read']]);
    expect(drop.props('accept')).toBe('.json,application/json');
  });

  it('passes the accepted types and the size limit to the drop zone, and refuses a larger file', async () => {
    const wrapper = mountDialog({ maxBytes: 1024 * 1024 });
    const drop = wrapper.getComponent(BaseFileDrop);
    expect(drop.props('maxBytes')).toBe(1024 * 1024);
    expect(drop.props('tooLargeLabel')).toBe('That file is larger than 1 MB.');
    const input = wrapper.get('input[type="file"]');
    const big = new File(['x'.repeat(2048)], 'big.json', { type: 'application/json' });
    Object.defineProperty(big, 'size', { value: 2 * 1024 * 1024 });
    Object.defineProperty(input.element, 'files', { value: [big], configurable: true });
    await input.trigger('change');
    await flushPromises();
    expect(wrapper.emitted('error')).toEqual([['That file is larger than 1 MB.']]);
    expect(wrapper.emitted('content')).toBeUndefined();
  });

  it('lists the staged items with a clear icon and enables the import only with something staged', async () => {
    const empty = mountDialog();
    expect(empty.find('.base-import-dialog__staged').exists()).toBe(false);
    expect(empty.get('.base-button--primary').attributes('disabled')).toBeDefined();
    const wrapper = mountDialog({ staged });
    const section = wrapper.get('.base-import-dialog__staged');
    expect(section.text()).toContain('Ready to import: 2');
    expect(section.findAll('li').map((item) => item.text())).toEqual(['First 3 rules', 'Second']);
    expect(wrapper.get('.base-button--primary').attributes('disabled')).toBeUndefined();
    await section.get('button[aria-label="Clear"]').trigger('click');
    expect(wrapper.emitted('clear')).toHaveLength(1);
    await wrapper.get('.base-button--primary').trigger('click');
    expect(wrapper.emitted('import')).toHaveLength(1);
  });

  it('is busy: the drop zone, clear and import are disabled and the dialog says so', async () => {
    const wrapper = mountDialog({ staged, busy: true });
    expect(wrapper.get('.base-import-dialog').attributes('aria-busy')).toBe('true');
    expect(wrapper.getComponent(BaseFileDrop).props('disabled')).toBe(true);
    expect(wrapper.get('.base-button--primary').attributes('disabled')).toBeDefined();
    expect(wrapper.get('button[aria-label="Clear"]').attributes('disabled')).toBeDefined();
  });

  it('announces errors and the success message', async () => {
    const wrapper = mountDialog({ errors: ['Not a template', 'Too many'], success: 'Imported 2.' });
    expect(wrapper.findAll('[role="alert"]').map((item) => item.text())).toEqual(['Not a template', 'Too many']);
    expect(wrapper.get('[role="status"]').text()).toBe('Imported 2.');
  });

  it('draws no tab bar for a single source or a step that replaces the body', async () => {
    const single = mountDialog({ tabs: [tabs[2]!] });
    expect(single.find('[role="tablist"]').exists()).toBe(false);
    expect(single.findComponent(BaseFileDrop).exists()).toBe(true);
    const hidden = mountDialog({ tab: 'local', hideTabs: true });
    expect(hidden.find('[role="tablist"]').exists()).toBe(false);
    expect(hidden.find('.local-body').exists()).toBe(true);
  });

  it('lets the owner replace the footer and the manual lead', async () => {
    const wrapper = mountWithPlugins(BaseImportDialog, {
      props: { open: true, tab: 'local', title: 'T', tabs },
      slots: { local: 'x', footer: '<button class="own">Mine</button>', 'manual-lead': '<p class="lead">Lead</p>' },
      attachTo: document.body,
    });
    expect(wrapper.find('.own').exists()).toBe(true);
    expect(wrapper.find('.base-button--primary').exists()).toBe(false);
    await wrapper.setProps({ tab: 'manual' });
    expect(wrapper.get('.lead').text()).toBe('Lead');
  });

  it('closes from the footer and keeps the open model in the owner', async () => {
    const Host = defineComponent({
      setup() {
        const open = ref(true);
        const tab = ref('manual');
        return () => h('div', [
          h(BaseImportDialog, { open: open.value, 'onUpdate:open': (value: boolean) => { open.value = value; }, tab: tab.value, 'onUpdate:tab': (value: string) => { tab.value = value; }, title: 'T', tabs }),
          h('output', String(open.value)),
        ]);
      },
    });
    const wrapper = mountWithPlugins(Host, { attachTo: document.body });
    const buttons = wrapper.findAll('.base-dialog__footer button');
    await buttons.at(-1)!.trigger('click');
    expect(wrapper.get('output').text()).toBe('false');
  });
});
