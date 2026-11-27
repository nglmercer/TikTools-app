import { expect, it } from 'vitest';
import { mergePluginGuides, pluginGuideCatalogItems, pluginGuideTour } from '../../../../../../src/web/features/guided-help/model/plugin-guides.ts';
import { ACTION_TYPE, PLUGIN_ID, defined } from '../../../../../shared/index.ts';

const type = {
  id: PLUGIN_ID.sonicboomServerSpeak,
  tag: 'tts',
  title: { default: 'Speak with SonicBoom', i18key: '' },
  description: { default: 'Speak', i18key: '' },
  source: { kind: 'plugin' as const, pluginId: PLUGIN_ID.sonicboomServer },
  requiredCapabilities: ['http.request'],
  fields: [
    { key: 'text', label: { default: 'Text', i18key: '' }, kind: 'text' as const, value: '' },
    { key: 'voice', label: { default: 'Voice', i18key: '' }, kind: 'select' as const, value: '' },
  ],
};

const snapshot = {
  actions: [],
  events: [],
  actionTypes: [type],
  plugins: [{ installed: true, enabled: true, descriptor: { id: PLUGIN_ID.sonicboomServer } }],
  translations: {},
};

function guide(steps: unknown[] = [{
  id: 'text',
  target: { kind: 'field', key: 'text' },
  title: { default: 'Text', i18key: '' },
  description: { default: 'Message template', i18key: '' },
}]) {
  return {
    id: 'sonicboom.server/speak-action',
    kind: 'automation-action',
    pluginId: PLUGIN_ID.sonicboomServer,
    actionType: PLUGIN_ID.sonicboomServerSpeak,
    mode: 'both',
    title: { default: 'Create a voice action', i18key: '' },
    description: { default: 'Configure speech', i18key: '' },
    steps,
  };
}

it('parses host-stamped plugin guides and converts field targets to semantic tour steps', () => {
  const [merged] = mergePluginGuides({ ...snapshot, pluginGuides: [guide()] });
  expect(merged?.id).toBe('sonicboom.server/speak-action');
  const dynamic = pluginGuideTour(defined(merged));
  expect(dynamic.definition.requirements?.automationActionType).toEqual({ typeId: PLUGIN_ID.sonicboomServerSpeak });
  expect(dynamic.definition.requirements?.automationActionExample).toBeUndefined();
  expect(dynamic.definition.steps[0]).toEqual(expect.objectContaining({
    target: 'action-field:text',
    missingTargetPolicy: 'cancel',
    prepareActions: ['automation.openOrCreateActionType:sonicboom.server.speak', 'automation.revealActionField'],
  }));
  expect(dynamic.texts['sonicboom.server/speak-action.text.title']).toBe('Text');
});

it('rejects field steps that do not exist in the action schema', () => {
  const invalid = guide([{
    id: 'missing',
    target: { kind: 'field', key: 'missing' },
    title: { default: 'Missing', i18key: '' },
    description: { default: 'Invalid field', i18key: '' },
  }]);
  expect(mergePluginGuides({ ...snapshot, pluginGuides: [invalid] })).toEqual([]);
});

it('rejects guides that are not namespaced or do not own their action type', () => {
  const foreignType = {
    ...snapshot,
    actionTypes: [{ ...type, id: ACTION_TYPE.coreFetch, source: { kind: 'builtin' as const } }],
    pluginGuides: [{ ...guide(), actionType: ACTION_TYPE.coreFetch }],
  };
  expect(mergePluginGuides(foreignType)).toEqual([]);
  const unnamespaced = { ...snapshot, pluginGuides: [{ ...guide(), id: 'other.plugin/speak-action' }] };
  expect(mergePluginGuides(unnamespaced)).toEqual([]);
});

it('catalog items require an active type but not an existing action for creation-oriented guides', () => {
  const [item] = pluginGuideCatalogItems({ ...snapshot, pluginGuides: [guide()] });
  expect(item?.requirementsMet).toBe(true);
  const inactive = pluginGuideCatalogItems({
    ...snapshot,
    plugins: [{ installed: true, enabled: false, descriptor: { id: PLUGIN_ID.sonicboomServer } }],
    pluginGuides: [guide()],
  });
  expect(inactive[0]?.requirementsMet).toBe(false);
});
