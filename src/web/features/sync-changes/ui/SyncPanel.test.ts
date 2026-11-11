import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { allCapabilities, fakeAdapter } from '../testing/fake-adapter.ts';
import type { SyncCapabilities } from '@/entities/cloud-sync/api/sync-adapter.ts';
import SyncPanel from './SyncPanel.vue';

enableAutoUnmount(afterEach);

async function open(capabilities: Partial<SyncCapabilities>) {
  const { adapter, spies } = fakeAdapter(capabilities);
  const wrapper = mountWithPlugins(SyncPanel, { attachTo: document.body, props: { adapter, open: false, 'onUpdate:open': (value: boolean) => wrapper.setProps({ open: value }) } });
  await wrapper.setProps({ open: true });
  await flushPromises();
  return { wrapper, spies, adapter };
}

const byLabel = (wrapper: VueWrapper, label: string) => wrapper.find(`[aria-label="${label}"]`);
const byText = (wrapper: VueWrapper, text: string) => wrapper.findAll('button').find((button) => button.text().includes(text));

describe('SyncPanel capabilities', () => {
  it('hides every control the adapter does not declare', async () => {
    const { wrapper } = await open({});
    expect(byLabel(wrapper, 'Fetch changes from Cloud').exists()).toBe(false);
    expect(byLabel(wrapper, 'Send the selected elements').exists()).toBe(false);
    expect(byLabel(wrapper, 'More actions').exists()).toBe(false);
    expect(byText(wrapper, 'Sync')).toBeUndefined();
    expect(wrapper.find('[role="switch"]').exists()).toBe(false);
    expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false);
    expect(byLabel(wrapper, 'Reset to the synced version').exists()).toBe(false);
    expect(wrapper.find('[role="radiogroup"][aria-label="Changes or history"]').exists()).toBe(false);
    // The list and the diff still work.
    expect(wrapper.findAll('[role="listitem"]')).toHaveLength(4);
  });

  it('shows the controls of the declared capabilities, and the fork entry only with the fork capability', async () => {
    const full = await open(allCapabilities);
    expect(byLabel(full.wrapper, 'Fetch changes from Cloud').exists()).toBe(true);
    expect(byLabel(full.wrapper, 'Send the selected elements').exists()).toBe(true);
    expect(byText(full.wrapper, 'Sync')).toBeDefined();
    expect(full.wrapper.find('[role="switch"]').exists()).toBe(true);
    await byLabel(full.wrapper, 'More actions').trigger('click');
    expect(full.wrapper.findAll('[role="menuitem"]').map((item) => item.text())).toEqual([
      'Force send (overwrites Cloud)', 'Force fetch (overwrites local)', 'Fork a public template…', 'Reset everything to the synced version',
    ]);
    full.wrapper.unmount();

    const noFork = await open({ ...allCapabilities, fork: false });
    await byLabel(noFork.wrapper, 'More actions').trigger('click');
    expect(noFork.wrapper.findAll('[role="menuitem"]').map((item) => item.text()).some((text) => text.includes('Fork'))).toBe(false);
  });

  it('runs the fork flow through the adapter', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byLabel(wrapper, 'More actions').trigger('click');
    await wrapper.findAll('[role="menuitem"]').find((item) => item.text().includes('Fork'))!.trigger('click');
    await flushPromises();
    expect(spies.fork).toHaveBeenCalledOnce();
  });
});

describe('SyncPanel changes view', () => {
  it('groups elements as To send, To fetch and Conflicts and counts them in the status strip', async () => {
    const { wrapper } = await open(allCapabilities);
    const groups = wrapper.findAll('.sync-panel__group').map((group) => group.text());
    expect(groups).toEqual(['To send 2', 'To fetch 1', 'Conflicts 1']);
    expect(wrapper.findAll('.sync-panel__status .sync-panel__chip').map((chip) => [chip.attributes('data-side'), chip.text()])).toEqual([['local', '2'], ['remote', '1'], ['conflict', '1']]);
  });

  it('sends only the checked elements', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byLabel(wrapper, 'Send Gift').setValue(false);
    await byLabel(wrapper, 'Send the selected elements').trigger('click');
    await flushPromises();
    expect(spies.push).toHaveBeenCalledWith(['e1']);
  });

  it('selects and clears everything with the select-all checkbox', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    const all = byLabel(wrapper, 'Select everything to send');
    await all.setValue(false);
    expect(byLabel(wrapper, 'Send the selected elements').attributes('disabled')).toBeDefined();
    await all.setValue(true);
    await byLabel(wrapper, 'Send the selected elements').trigger('click');
    await flushPromises();
    expect(spies.push).toHaveBeenCalledWith(['e1', 'e2']);
  });

  it('shows the diff of the picked element', async () => {
    const { wrapper } = await open(allCapabilities);
    await byLabel(wrapper, 'View Gift').trigger('click');
    expect(wrapper.get('.sync-panel__head').text()).toContain('Gift');
    expect(wrapper.get('.sync-panel__head').text()).toContain('Added');
    expect(wrapper.find('.base-json-diff__line--added').exists()).toBe(true);
  });

  it('syncs and fetches through the adapter without confirmation', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byText(wrapper, 'Sync')!.trigger('click');
    await byLabel(wrapper, 'Fetch changes from Cloud').trigger('click');
    await flushPromises();
    expect(spies.sync).toHaveBeenCalledOnce();
    expect(spies.fetch).toHaveBeenCalledOnce();
  });
});

describe('SyncPanel destructive confirmation', () => {
  it.each([
    ['Force send (overwrites Cloud)', 'forcePush'],
    ['Force fetch (overwrites local)', 'forcePull'],
    ['Reset everything to the synced version', 'resetAll'],
  ] as const)('asks before "%s" and runs it only after confirming', async (label, spy) => {
    const { wrapper, spies } = await open(allCapabilities);
    await byLabel(wrapper, 'More actions').trigger('click');
    await wrapper.findAll('[role="menuitem"]').find((item) => item.text() === label)!.trigger('click');
    expect(spies[spy]).not.toHaveBeenCalled();
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(true);
    await byText(wrapper, 'Cancel')!.trigger('click');
    await flushPromises();
    expect(spies[spy]).not.toHaveBeenCalled();
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false);

    await byLabel(wrapper, 'More actions').trigger('click');
    await wrapper.findAll('[role="menuitem"]').find((item) => item.text() === label)!.trigger('click');
    await byText(wrapper, 'Confirm')!.trigger('click');
    await flushPromises();
    expect(spies[spy]).toHaveBeenCalledOnce();
  });

  it('asks before resetting one element', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byLabel(wrapper, 'Reset to the synced version').trigger('click');
    expect(spies.reset).not.toHaveBeenCalled();
    expect(wrapper.get('[role="alertdialog"]').text()).toContain('Greeting');
    await byText(wrapper, 'Confirm')!.trigger('click');
    await flushPromises();
    expect(spies.reset).toHaveBeenCalledWith('e1');
  });
});

describe('SyncPanel conflicts', () => {
  async function conflict(capabilities: Partial<SyncCapabilities> = allCapabilities) {
    const context = await open(capabilities);
    await byLabel(context.wrapper, 'View Clash').trigger('click');
    return context;
  }

  it('applies the per-field choice as one merged object', async () => {
    const { wrapper, spies } = await conflict();
    expect(wrapper.findAll('.sync-panel__field').map((field) => field.find('.sync-panel__key').text())).toEqual(['name', 'volume', 'color']);
    // name: both changed -> Local by default; volume: only Cloud changed -> Cloud; color: only Local changed -> Local.
    await wrapper.get('[role="radiogroup"][aria-label="name"] [role="radio"]:nth-child(3)').trigger('click');
    await wrapper.get('[role="radiogroup"][aria-label="color"] [role="radio"]:nth-child(1)').trigger('click');
    await byText(wrapper, 'Apply merge')!.trigger('click');
    await flushPromises();
    expect(spies.resolve).toHaveBeenCalledWith('e4', { choice: 'merged', value: { name: 'Theirs', volume: 9, color: 'red' } });
  });

  it('"All local" and "All Cloud" set every field before applying', async () => {
    const { wrapper, spies } = await conflict();
    await byText(wrapper, 'All Cloud')!.trigger('click');
    await byText(wrapper, 'Apply merge')!.trigger('click');
    await flushPromises();
    expect(spies.resolve).toHaveBeenLastCalledWith('e4', { choice: 'merged', value: { name: 'Theirs', volume: 9, color: 'red' } });
    await byText(wrapper, 'All local')!.trigger('click');
    await byText(wrapper, 'Apply merge')!.trigger('click');
    await flushPromises();
    expect(spies.resolve).toHaveBeenLastCalledWith('e4', { choice: 'merged', value: { name: 'Mine', volume: 5, color: 'blue' } });
  });

  it('without the merge capability offers keep-local and take-Cloud only', async () => {
    const { wrapper, spies } = await conflict({ resolve: true });
    expect(wrapper.find('.sync-panel__field').exists()).toBe(false);
    expect(byText(wrapper, 'Apply merge')).toBeUndefined();
    await byText(wrapper, 'All Cloud')!.trigger('click');
    await flushPromises();
    expect(spies.resolve).toHaveBeenCalledWith('e4', { choice: 'cloud' });
  });

  it('shows no resolution controls when the adapter cannot resolve', async () => {
    const { wrapper } = await conflict({});
    expect(wrapper.find('.sync-panel__conflict').exists()).toBe(false);
  });
});

describe('SyncPanel history view', () => {
  it('lists operations, shows the changed elements and diff of the selected one', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byText(wrapper, 'History')!.trigger('click');
    await flushPromises();
    expect(spies.history).toHaveBeenCalled();
    expect(wrapper.findAll('.sync-panel__left [role="listitem"]')).toHaveLength(2);
    expect(wrapper.get('.sync-panel__head').text()).toContain('Sent two events');
    expect(wrapper.findAll('.sync-panel__files [role="listitem"]')).toHaveLength(2);
    expect(wrapper.find('.base-json-diff').exists()).toBe(true);
    await wrapper.findAll('.sync-panel__left .sync-panel__pick')[1]!.trigger('click');
    expect(wrapper.get('.sync-panel__head').text()).toContain('Fetched one event');
  });

  it('confirms before restoring one element or reverting the operation', async () => {
    const { wrapper, spies } = await open(allCapabilities);
    await byText(wrapper, 'History')!.trigger('click');
    await flushPromises();
    await wrapper.findAll('[aria-label="Restore this version"]')[0]!.trigger('click');
    expect(spies.restore).not.toHaveBeenCalled();
    await byText(wrapper, 'Confirm')!.trigger('click');
    await flushPromises();
    expect(spies.restore).toHaveBeenCalledWith('op1', 'e1');
    await byLabel(wrapper, 'Revert this operation').trigger('click');
    expect(spies.restore).toHaveBeenCalledTimes(1);
    await byText(wrapper, 'Confirm')!.trigger('click');
    await flushPromises();
    expect(spies.restore).toHaveBeenLastCalledWith('op1');
  });

  it('has no History view without the history capability', async () => {
    const { wrapper } = await open({ ...allCapabilities, history: false });
    expect(byText(wrapper, 'History')).toBeUndefined();
  });
});
