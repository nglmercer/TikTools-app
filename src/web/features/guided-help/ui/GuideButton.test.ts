import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import GuideButton from './GuideButton.vue';

type TestConfig = Record<string, unknown> & { onDestroyed?: () => void; allowKeyboardControl?: boolean };
const fake = vi.hoisted(() => {
  let config: TestConfig = {};
  const engine = { setSteps: vi.fn<(steps: Array<{ element: Element; popover: { description: string } }>) => void>(), drive: vi.fn(), getConfig: () => config, setConfig: vi.fn((next: TestConfig) => { config = next; }), destroy: vi.fn(() => config.onDestroyed?.()) };
  return { engine, driver: vi.fn((options: TestConfig) => { config = options; return engine; }), config: () => config };
});
vi.mock('driver.js', () => ({ driver: fake.driver }));
enableAutoUnmount(afterEach);
beforeEach(() => { vi.clearAllMocks(); sessionStorage.clear(); });
afterEach(() => { document.querySelectorAll('[data-test-guide]').forEach(element => element.remove()); });

async function mountGuide() {
  const root = document.createElement('section');
  root.setAttribute('data-guide-scope', 'test'); root.setAttribute('data-test-guide', '');
  root.innerHTML = '<button id="one">First</button><button id="two">Second</button><button id="disabled" disabled>Disabled</button>';
  document.body.append(root);
  const wrapper = mountWithPlugins(GuideButton, { attachTo: root, props: { id: 'test-v1', steps: [{ id: 'one', selector: '#one', title: 'First', description: '<safe>' }, { id: 'disabled', selector: '#disabled', title: 'Unavailable', description: '' }, { id: 'absent', selector: '#absent', title: 'Absent', description: '' }] } });
  return { wrapper, root };
}
async function open(wrapper: Awaited<ReturnType<typeof mountGuide>>['wrapper']) {
  await wrapper.get('button').trigger('click'); await vi.dynamicImportSettled(); await flushPromises();
}
it('loads the tour only when requested, skips unavailable controls and escapes text', async () => {
  const { wrapper, root } = await mountGuide();
  expect(fake.driver).not.toHaveBeenCalled();
  await open(wrapper);
  const steps = fake.engine.setSteps.mock.calls[0]![0];
  expect(steps).toHaveLength(1);
  expect(steps[0]!.element).toBe(root.querySelector('#one'));
  expect(steps[0]!.popover.description).toBe('&lt;safe&gt;');
  expect(fake.config().allowKeyboardControl).toBe(true);
});
it('tracks a completed application action through props and cleans up on unmount', async () => {
  const { wrapper, root } = await mountGuide();
  await open(wrapper);
  await wrapper.setProps({ steps: [{ id: 'two', selector: '#two', title: 'Second', description: 'Ready' }] });
  await flushPromises();
  expect(fake.engine.setSteps.mock.lastCall![0][0]!.element).toBe(root.querySelector('#two'));
  wrapper.unmount();
  expect(fake.engine.destroy).toHaveBeenCalled();
});
it('closes the tour when a native dialog opens so it cannot trap focus behind the dialog', async () => {
  const { wrapper, root } = await mountGuide();
  await open(wrapper);
  const dialog = document.createElement('dialog'); root.append(dialog); dialog.setAttribute('open', '');
  await flushPromises();
  expect(fake.engine.destroy).toHaveBeenCalled();
});
