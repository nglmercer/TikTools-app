import { flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { startTour } from '../model/coordinator.ts';
import TutorialCatalogDialog from './TutorialCatalogDialog.vue';

vi.mock('../model/coordinator.ts', () => ({ startTour: vi.fn() }));

beforeEach(async () => {
  await activateLocale('en');
  vi.clearAllMocks();
});
afterEach(async () => { await activateLocale('en'); });

function mountDialog() {
  return mountWithPlugins(TutorialCatalogDialog, { attachTo: document.body, props: { open: true } });
}

it('lists every tutorial and filters by search and the category select', async () => {
  const wrapper = mountDialog();
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(8);
  expect(wrapper.find('[role="combobox"]').exists()).toBe(true);

  await wrapper.get('input[type="search"]').setValue('plugins');
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(1);
  await wrapper.get('input[type="search"]').setValue('');

  await wrapper.get('[role="combobox"]').trigger('click');
  await wrapper.findAll('[role="option"]').find((option) => option.text() === 'Events')!.trigger('click');
  const titles = wrapper.findAll('.tutorial-catalog__card strong').map((node) => node.text());
  expect(titles).toEqual(['Automations', 'Library changes']);
  wrapper.unmount();
});

it('opens on the category requested by the caller', () => {
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, initialCategory: 'feed' },
  });
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(1);
  expect(wrapper.get('.tutorial-catalog__card strong').text()).toBe('Live feed');
  wrapper.unmount();
});

it('shows the steps in the detail panel and starts the selected driver.js tour', async () => {
  const wrapper = mountDialog();
  expect(wrapper.findAll('.tutorial-catalog__step-list li').length).toBeGreaterThan(0);
  await wrapper.findAll('.tutorial-catalog__card')[1]!.trigger('click');
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe('Library changes');

  await wrapper.get('.tutorial-catalog__detail button').trigger('click');
  await flushPromises();
  expect(startTour).toHaveBeenCalledWith('changes', expect.objectContaining({ locale: 'en' }));
  expect(wrapper.emitted('update:open')?.at(-1)?.[0]).toBe(false);
  wrapper.unmount();
});
