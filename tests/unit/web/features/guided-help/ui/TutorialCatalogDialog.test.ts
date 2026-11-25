import { flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { testMessage } from '@/shared/i18n/test-message.ts';
import { startTour } from '../../../../../../src/web/features/guided-help/model/coordinator.ts';
import { tourRegistry } from '../../../../../../src/web/features/guided-help/model/registry.ts';
import { tutorialCard, tutorialEntries } from '../../../../../../src/web/features/guided-help/model/tutorial-catalog.ts';
import type { TourId } from '../../../../../../src/web/features/guided-help/model/types.ts';
import TutorialCatalogDialog from '../../../../../../src/web/features/guided-help/ui/TutorialCatalogDialog.vue';

vi.mock('../../../../../../src/web/features/guided-help/model/coordinator.ts', () => ({ startTour: vi.fn() }));

beforeEach(async () => {
  await activateLocale('en');
  vi.clearAllMocks();
});
afterEach(async () => { await activateLocale('en'); });

function mountDialog() {
  return mountWithPlugins(TutorialCatalogDialog, { attachTo: document.body, props: { open: true } });
}

const titleOf = (id: TourId): string => tutorialCard('en', id).title;
const entriesByTag = (tag: string) => tutorialEntries.filter((entry) => entry.tags.includes(tag as never));

/** The catalog paginates six cards per page: walk every page collecting card titles. */
async function collectCardTitles(wrapper: ReturnType<typeof mountDialog>): Promise<string[]> {
  const titles: string[] = [];
  for (;;) {
    for (const node of wrapper.findAll('.tutorial-catalog__card strong')) titles.push(node.text());
    const next = wrapper.findAll('.tutorial-catalog__pager button').at(-1);
    if (!next || next.attributes('disabled') !== undefined) return titles;
    await next.trigger('click');
    await flushPromises();
  }
}

async function clickCardByTitle(wrapper: ReturnType<typeof mountDialog>, title: string): Promise<void> {
  for (;;) {
    const card = wrapper.findAll('.tutorial-catalog__card').find((node) => node.text().includes(title));
    if (card) { await card.trigger('click'); return; }
    const next = wrapper.findAll('.tutorial-catalog__pager button').at(-1);
    if (!next || next.attributes('disabled') !== undefined) throw new Error(`card not found: ${title}`);
    await next.trigger('click');
    await flushPromises();
  }
}

it('lists every tutorial and filters by search and the category select', async () => {
  const wrapper = mountDialog();
  expect(await collectCardTitles(wrapper)).toHaveLength(tutorialEntries.length);
  expect(wrapper.find('[role="combobox"]').exists()).toBe(true);

  await wrapper.get('input[type="search"]').setValue('plugins');
  expect(wrapper.findAll('.tutorial-catalog__card').length).toBeGreaterThanOrEqual(entriesByTag('plugins').length);
  await wrapper.get('input[type="search"]').setValue('');

  await wrapper.get('[role="combobox"]').trigger('click');
  await wrapper.findAll('[role="option"]').find((option) => option.text() === testMessage('en', 'guides.catalog.tags.events'))!.trigger('click');
  const titles = await collectCardTitles(wrapper);
  expect(titles.sort()).toEqual(entriesByTag('events').map((entry) => titleOf(entry.id)).sort());
  wrapper.unmount();
});

it('opens on the category requested by the caller', () => {
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, initialCategory: 'feed' },
  });
  const feedEntries = entriesByTag('feed');
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(feedEntries.length);
  expect(wrapper.get('.tutorial-catalog__card strong').text()).toBe(titleOf(feedEntries[0]!.id));
  wrapper.unmount();
});

it('limits the catalog to the caller tours and preselects the recommended one', async () => {
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, tours: ['connection', 'feed'], initialTour: 'feed' },
  });
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(2);
  expect(wrapper.get('.tutorial-catalog__card--active strong').text()).toBe(titleOf('feed'));
  await wrapper.get('.tutorial-catalog__detail button').trigger('click');
  await flushPromises();
  expect(startTour).toHaveBeenCalledWith('feed', expect.objectContaining({ locale: 'en' }));
  wrapper.unmount();
});

it('delegates launching when the caller supplies a handler', async () => {
  const onLaunch = vi.fn();
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, onLaunch },
  });
  await wrapper.get('.tutorial-catalog__detail button').trigger('click');
  await flushPromises();
  expect(onLaunch).toHaveBeenCalledWith(tutorialEntries[0]!.id);
  expect(startTour).not.toHaveBeenCalled();
  expect(wrapper.emitted('update:open')?.at(-1)?.[0]).toBe(false);
  wrapper.unmount();
});

it('shows the registry steps in the detail panel and starts the selected driver.js tour', async () => {
  const wrapper = mountDialog();
  const first = tutorialEntries[0]!.id;
  expect(wrapper.findAll('.tutorial-catalog__step-list li')).toHaveLength(tourRegistry[first].steps.length);

  await clickCardByTitle(wrapper, titleOf('changes'));
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe(titleOf('changes'));
  expect(wrapper.findAll('.tutorial-catalog__step-list li')).toHaveLength(tourRegistry.changes.steps.length);

  await wrapper.get('.tutorial-catalog__detail button').trigger('click');
  await flushPromises();
  expect(startTour).toHaveBeenCalledWith('changes', expect.objectContaining({ locale: 'en' }));
  expect(wrapper.emitted('update:open')?.at(-1)?.[0]).toBe(false);
  wrapper.unmount();
});
