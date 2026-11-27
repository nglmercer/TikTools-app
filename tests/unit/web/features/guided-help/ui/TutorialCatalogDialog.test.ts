import { flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { testMessage } from '@/shared/i18n/test-message.ts';
import { toElement } from '../../../../support/dom.ts';
import { startDynamicTour, startTour } from '../../../../../../src/web/features/guided-help/model/coordinator.ts';
import { tourRegistry } from '../../../../../../src/web/features/guided-help/model/registry.ts';
import { tutorialCard, tutorialCategories, tutorialEntries, tutorialSteps } from '../../../../../../src/web/features/guided-help/model/tutorial-catalog.ts';
import TutorialCatalogDialog from '../../../../../../src/web/features/guided-help/ui/TutorialCatalogDialog.vue';
import { ACTION_TYPE, PLUGIN_ID } from '../../../../../shared/index.ts';

vi.mock('../../../../../../src/web/features/guided-help/model/coordinator.ts', () => ({ startTour: vi.fn(), startDynamicTour: vi.fn() }));

beforeEach(async () => {
  await activateLocale('en');
  vi.clearAllMocks();
});
afterEach(async () => { await activateLocale('en'); });

function mountDialog() {
  return mountWithPlugins(TutorialCatalogDialog, { attachTo: document.body, props: { open: true } });
}

const titleOf = (id: string): string => tutorialCard('en', id).title;
const entriesByTag = (tag: string) => tutorialEntries.filter((entry) => entry.tags.some((candidate) => candidate === tag));
const entriesByCategory = (category: string) => tutorialEntries.filter((entry) => entry.category === category);
const automationEntriesByTag = (tag: string) => entriesByCategory('automations').filter((entry) => entry.tags.some((candidate) => candidate === tag));

async function openTutorialByTitle(wrapper: ReturnType<typeof mountDialog>, title: string): Promise<void> {
  await wrapper.get('input[type="search"]').setValue(title);
  await flushPromises();
  const card = wrapper.findAll('.tutorial-catalog__card').find((node) => node.text().includes(title));
  if (!card) throw new Error(`card not found: ${title}`);
  await card.trigger('click');
}

async function openCategory(wrapper: ReturnType<typeof mountDialog>, tag: string): Promise<void> {
  const label = testMessage('en', `guides.catalog.tags.${tag}`);
  const card = wrapper.findAll('.tutorial-catalog__category-card').find((node) => node.text().includes(label));
  if (!card) throw new Error(`category not found: ${tag}`);
  await card.trigger('click');
}

it('opens a catalog of category cards with the icon category select', () => {
  const wrapper = mountDialog();
  expect(wrapper.find('[role="combobox"]').exists()).toBe(true);
  expect(wrapper.find('.tutorial-catalog__pager').exists()).toBe(false);
  expect(wrapper.find('.tutorial-catalog__screen--catalog').exists()).toBe(true);
  expect(wrapper.findAll('.tutorial-catalog__category-card')).toHaveLength(tutorialCategories.length);
  expect(wrapper.get('.tutorial-catalog__wizard').text()).toContain(testMessage('en', 'guides.catalog.screens.catalog'));
  wrapper.unmount();
});

it('groups automations by product area and filters them by their real tabs', async () => {
  const wrapper = mountDialog();
  await openCategory(wrapper, 'automations');

  expect(wrapper.find('.tutorial-catalog__screen--category').exists()).toBe(true);
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(entriesByCategory('automations').length);
  expect(wrapper.findAll('.tutorial-catalog__section--overview .tutorial-catalog__card')).toHaveLength(3);
  expect(wrapper.text()).toContain(testMessage('en', 'guides.catalog.overview'));
  expect(wrapper.text()).toContain(testMessage('en', 'guides.catalog.specializedGuides'));

  await wrapper.get('[role="combobox"]').trigger('click');
  await flushPromises();
  const options = Array.from(document.querySelectorAll<HTMLElement>('[role="option"]'));
  expect(options.map((option) => option.textContent?.trim())).toEqual([
    testMessage('en', 'guides.catalog.allTags'),
    testMessage('en', 'guides.catalog.tags.events'),
    testMessage('en', 'guides.catalog.tags.actions'),
    testMessage('en', 'guides.catalog.tags.variables'),
  ]);
  options.find((option) => option.textContent?.includes(testMessage('en', 'guides.catalog.tags.actions')))?.click();
  await flushPromises();

  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(automationEntriesByTag('actions').length);
  expect(wrapper.get('.tutorial-catalog__card--featured strong').text()).toBe(testMessage('en', 'guides.catalog.tags.actions'));
  wrapper.unmount();
});

it('uses distinct SVG metaphors for the automation guides', () => {
  const icons = entriesByCategory('automations').map((entry) => entry.icon);
  expect(new Set(icons).size).toBe(icons.length);
});

it('searches across every category and opens the matching guide detail', async () => {
  const wrapper = mountDialog();
  await wrapper.get('input[type="search"]').setValue('plugins');
  await flushPromises();

  const titles = wrapper.findAll('.tutorial-catalog__card strong').map((node) => node.text());
  for (const entry of entriesByTag('plugins')) {
    expect(titles.some((shown) => shown.includes(titleOf(entry.id)))).toBe(true);
  }
  await openTutorialByTitle(wrapper, titleOf('plugin'));
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe(titleOf('plugin'));
  wrapper.unmount();
});

it('opens on the category requested by the caller', () => {
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, initialCategory: 'feed' },
  });
  const feedEntries = entriesByCategory('feed');
  expect(wrapper.find('.tutorial-catalog__screen--category').exists()).toBe(true);
  expect(wrapper.findAll('.tutorial-catalog__card')).toHaveLength(feedEntries.length);
  expect(wrapper.get('.tutorial-catalog__card--featured strong').text()).toContain(titleOf('feed'));
  wrapper.unmount();
});

it('navigates catalog, category and detail as a three-step wizard', async () => {
  const wrapper = mountDialog();
  await openCategory(wrapper, 'automations');
  await openTutorialByTitle(wrapper, titleOf('automationEvents'));

  expect(wrapper.find('.tutorial-catalog__screen--detail').exists()).toBe(true);
  expect(wrapper.get('.tutorial-catalog__wizard').text()).toContain(testMessage('en', 'guides.catalog.screens.guide'));

  await wrapper.get('.tutorial-catalog__back').trigger('click');
  expect(wrapper.find('.tutorial-catalog__screen--category').exists()).toBe(true);
  expect(wrapper.find('.tutorial-catalog__screen--detail').exists()).toBe(false);

  await wrapper.get('.tutorial-catalog__wizard button').trigger('click');
  expect(wrapper.find('.tutorial-catalog__screen--catalog').exists()).toBe(true);
  expect(wrapper.find('.tutorial-catalog__screen--category').exists()).toBe(false);
  expect(wrapper.get('.tutorial-catalog__wizard').text()).not.toContain(testMessage('en', 'guides.catalog.tags.automations'));
  wrapper.unmount();
});

it('shows one step indicator with previous and next controls synchronized with the miniature demo', async () => {
  const wrapper = mountDialog();
  await openTutorialByTitle(wrapper, titleOf('automationEvents'));

  const steps = tutorialSteps('en', 'automationEvents');
  expect(wrapper.findAll('.tutorial-step-preview__numbers button')).toHaveLength(steps.length);
  expect(wrapper.find('.tutorial-demo-preview').exists()).toBe(true);
  expect(wrapper.get('.tutorial-step-preview__body').text()).toContain(steps[0]!.title);
  expect(wrapper.get('.tutorial-demo-preview__zone--toolbar').classes()).toContain('tutorial-demo-preview__zone--active');

  const controls = wrapper.findAll('.tutorial-step-preview__controls button');
  await controls.at(-1)!.trigger('click');
  expect(wrapper.get('.tutorial-step-preview__body').text()).toContain(steps[1]!.title);
  expect(wrapper.get('.tutorial-demo-preview__zone--tabs').classes()).toContain('tutorial-demo-preview__zone--active');
  expect(wrapper.get('.tutorial-step-preview__numbers button[aria-current=\'step\']').text()).toBe('2');

  await controls[0]!.trigger('click');
  expect(wrapper.get('.tutorial-step-preview__body').text()).toContain(steps[0]!.title);
  wrapper.unmount();
});

it('moves focus into detail, restores it to the card and supports step arrow keys', async () => {
  const wrapper = mountDialog();
  await wrapper.get('input[type="search"]').setValue(titleOf('automationEvents'));
  await flushPromises();
  const card = wrapper.findAll('.tutorial-catalog__card').find((node) => node.text().includes(titleOf('automationEvents')));
  if (!card) throw new Error('automation events card not found');
  toElement(card.element, HTMLElement).focus();
  await card.trigger('click');
  await flushPromises();

  expect(document.activeElement).toBe(wrapper.get('.tutorial-catalog__detail').element);

  const steps = tutorialSteps('en', 'automationEvents');
  const numbers = wrapper.findAll('.tutorial-step-preview__numbers button');
  toElement(numbers[1]!.element, HTMLElement).focus();
  await numbers[1]!.trigger('keydown', { key: 'ArrowRight' });
  await flushPromises();
  expect(wrapper.get('.tutorial-step-preview__body').text()).toContain(steps[2]!.title);
  expect(document.activeElement).toBe(numbers[2]!.element);

  await wrapper.get('.tutorial-catalog__back').trigger('click');
  await flushPromises();
  expect(toElement(document.activeElement, HTMLElement).dataset.tutorialId).toBe('automationEvents');
  wrapper.unmount();
});


it('filters signed-in cloud workspace tours only when the host reports an anonymous account', async () => {
  const signedOut = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, signedIn: false },
  });
  await signedOut.get('input[type="search"]').setValue(titleOf('backup'));
  await flushPromises();
  expect(signedOut.find('.tutorial-catalog__card').exists()).toBe(false);
  signedOut.unmount();

  const signedIn = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, signedIn: true },
  });
  await signedIn.get('input[type="search"]').setValue(titleOf('backup'));
  await flushPromises();
  expect(signedIn.findAll('.tutorial-catalog__card').some((card) => card.text().includes(titleOf('backup')))).toBe(true);
  signedIn.unmount();
});


it('shows action example requirements and blocks starting when the workspace lacks the record', async () => {
  const missing = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: {
      open: true,
      initialTour: 'automationActionTts',
      automationSnapshot: {
        actions: [{ typeId: ACTION_TYPE.coreLog }],
        actionTypes: [{ id: ACTION_TYPE.demoTts, tag: 'tts', title: { default: 'Speak', i18key: '' }, source: { kind: 'plugin', pluginId: 'demo.audio' } }],
        plugins: [{ installed: true, enabled: true, descriptor: { id: 'demo.audio' } }],
      },
    },
  });
  await flushPromises();
  expect(missing.get('.tutorial-catalog__requirement').text()).toContain('Requires an existing matching action');
  expect(missing.get('.tutorial-catalog__requirement-action').text()).toBe('Show me how');
  expect(missing.get('.tutorial-catalog__start').attributes('disabled')).toBeDefined();
  missing.unmount();

  const available = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: {
      open: true,
      initialTour: 'automationActionTts',
      automationSnapshot: {
        actions: [{ typeId: ACTION_TYPE.demoTts }],
        actionTypes: [{ id: ACTION_TYPE.demoTts, tag: 'tts', title: { default: 'Speak', i18key: '' }, source: { kind: 'plugin', pluginId: 'demo.audio' } }],
        plugins: [{ installed: true, enabled: true, descriptor: { id: 'demo.audio' } }],
      },
    },
  });
  await flushPromises();
  expect(available.find('.tutorial-catalog__requirement').exists()).toBe(false);
  expect(available.get('.tutorial-catalog__start').attributes('disabled')).toBeUndefined();
  available.unmount();
});

it('limits the catalog to the caller tours and preselects the recommended one', async () => {
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, tours: ['connection', 'feed'], initialTour: 'feed' },
  });
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe(titleOf('feed'));
  await wrapper.get('.tutorial-catalog__back').trigger('click');

  expect(wrapper.find('.tutorial-catalog__screen--category').exists()).toBe(true);
  expect(wrapper.findAll('.tutorial-catalog__category-card')).toHaveLength(0);
  await wrapper.get('.tutorial-catalog__wizard button').trigger('click');
  expect(wrapper.findAll('.tutorial-catalog__category-card')).toHaveLength(2);
  wrapper.unmount();
});

it('uses the host interactive demo frame when a preview URL is supplied', async () => {
  const previewUrl = vi.fn(() => '/demo/?scenario=connection#/connections');
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, initialTour: 'connection', previewUrl },
  });
  await flushPromises();

  expect(previewUrl).toHaveBeenCalledWith('connection');
  expect(wrapper.get('[data-testid="tutorial-demo-frame"]').attributes('src')).toContain('/demo/');
  expect(wrapper.find('.tutorial-demo-preview__window').exists()).toBe(false);
  wrapper.unmount();
});

it('delegates launching when the caller supplies a handler', async () => {
  const onLaunch = vi.fn();
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: { open: true, onLaunch, initialTour: tutorialEntries[0]!.id },
  });
  await wrapper.get('.tutorial-catalog__detail .tutorial-catalog__start').trigger('click');
  await flushPromises();
  expect(onLaunch).toHaveBeenCalledWith(tutorialEntries[0]!.id);
  expect(startTour).not.toHaveBeenCalled();
  expect(wrapper.emitted('update:open')?.at(-1)?.[0]).toBe(false);
  wrapper.unmount();
});

it('shows the registry steps in the detail screen and starts the selected driver.js tour', async () => {
  const wrapper = mountDialog();
  await openTutorialByTitle(wrapper, titleOf('changes'));
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe(titleOf('changes'));
  expect(wrapper.findAll('.tutorial-step-preview__numbers button')).toHaveLength(tourRegistry.changes.steps.length);

  await wrapper.get('.tutorial-catalog__detail .tutorial-catalog__start').trigger('click');
  await flushPromises();
  expect(startTour).toHaveBeenCalledWith('changes', expect.objectContaining({ locale: 'en' }));
  expect(wrapper.emitted('update:open')?.at(-1)?.[0]).toBe(false);
  wrapper.unmount();
});

it('lists a plugin-contributed action guide and starts it as a dynamic tour', async () => {
  // Arrange: a plugin action type with one text field, and a plugin guide that targets that field
  const textField = {
    key: 'text',
    label: { default: 'Text', i18key: '' },
    kind: 'text' as const,
    value: '{{ event.data.comment }}',
  };
  const wrapper = mountWithPlugins(TutorialCatalogDialog, {
    attachTo: document.body,
    props: {
      open: true,
      automationSnapshot: {
        actions: [],
        actionTypes: [{
          id: PLUGIN_ID.sonicboomServerSpeak,
          tag: 'tts',
          title: { default: 'Speak with SonicBoom', i18key: '' },
          source: { kind: 'plugin', pluginId: PLUGIN_ID.sonicboomServer },
          fields: [textField],
        }],
        plugins: [{ installed: true, enabled: true, descriptor: { id: PLUGIN_ID.sonicboomServer } }],
        pluginGuides: [{
          id: 'sonicboom.server/speak-action',
          kind: 'automation-action',
          pluginId: PLUGIN_ID.sonicboomServer,
          actionType: PLUGIN_ID.sonicboomServerSpeak,
          mode: 'both',
          title: { default: 'Create a SonicBoom voice action', i18key: '' },
          description: { default: 'Configure the spoken message.', i18key: '' },
          icon: 'volume',
          steps: [{
            id: 'text',
            target: { kind: 'field', key: 'text' },
            title: { default: 'Choose what is spoken', i18key: '' },
            description: { default: 'Write the message template.', i18key: '' },
          }],
        }],
      },
    },
  });
  // Act: search by the plugin's name, which must surface its guide next to the built-in tours
  await wrapper.get('input[type="search"]').setValue('SonicBoom');
  await flushPromises();

  const card = wrapper.findAll('.tutorial-catalog__card').find((node) => node.text().includes('Create a SonicBoom voice action'));
  expect(card).toBeDefined();
  await card!.trigger('click');
  await flushPromises();
  // Assert: the card opens the plugin guide's detail and shows its field step before anything starts
  expect(wrapper.get('.tutorial-catalog__detail h3').text()).toBe('Create a SonicBoom voice action');
  expect(wrapper.get('.tutorial-step-preview__body').text()).toContain('Choose what is spoken');

  // Act: start the guide; it must run as a dynamic tour bound to the plugin's action type
  await wrapper.get('.tutorial-catalog__start').trigger('click');
  await flushPromises();
  expect(startDynamicTour).toHaveBeenCalledWith(
    expect.objectContaining({
      definition: expect.objectContaining({
        id: 'sonicboom.server/speak-action',
        requirements: expect.objectContaining({ automationActionType: { typeId: PLUGIN_ID.sonicboomServerSpeak } }),
      }),
    }),
    expect.objectContaining({ locale: 'en' }),
  );
  expect(startTour).not.toHaveBeenCalled();
  wrapper.unmount();
});
