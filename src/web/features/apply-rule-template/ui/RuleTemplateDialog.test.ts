import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import type { ActionTypeDefinition } from '../../../../automation/behavior/types.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { parseRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import type { RuleTemplateTab } from '../model/template-tab.ts';
import RuleTemplateDialog from './RuleTemplateDialog.vue';

enableAutoUnmount(afterEach); afterEach(() => activateLocale('en'));
test('oversized expansion reports an error while retaining the configured template and parameters, then allows a bounded correction', async () => {
  await activateLocale('en');
  const parsed = parseRuleTemplate({ templateVersion: 1, id: 'bounded-template', title: 'Bounded template', params: { type: 'object', properties: { value: { type: 'string', title: 'Value', default: 'small' } } },
    actions: [{ name: 'Log', typeId: 'core.log', config: { message: '{{ params.value }}'.repeat(500) } }], event: { name: 'Chat', trigger: 'tiktok.chat' } });
  if (!parsed.ok) throw new Error(parsed.errors.join('; '));
  const wrapper = mountWithPlugins(RuleTemplateDialog, { props: { open: true, customTemplates: [{ ...parsed.template, custom: true }],
    snapshot: { actions: [], events: [], plugins: [], translations: {}, actionTypes: [{ id: 'core.log', title: { default: 'Log' }, source: { kind: 'builtin' }, requiredCapabilities: [] } as unknown as ActionTypeDefinition] } }, attachTo: document.body });
  await wrapper.findAll('.rule-templates__card').find((card) => card.text().includes('Bounded template'))!.trigger('click');
  const value = 'x'.repeat(256 * 1024); const form = wrapper.getComponent(BaseSchemaForm);
  form.vm.$emit('update:modelValue', { value }); await flushPromises();
  await wrapper.get('.base-button--primary').trigger('click');
  expect(wrapper.get('[role="alert"]').text()).toContain('expanded rules are too large');
  expect(form.props('modelValue')).toEqual({ value }); expect(wrapper.emitted('apply')).toBeUndefined(); expect(wrapper.get('dialog').attributes('open')).toBeDefined();
  form.vm.$emit('update:modelValue', { value: 'safe' }); await flushPromises();
  await wrapper.get('.base-button--primary').trigger('click');
  expect(wrapper.emitted('apply')).toHaveLength(1);
});

const snapshot = { actions: [], events: [], plugins: [], translations: {}, actionTypes: [{ id: 'core.log', title: { default: 'Log' }, source: { kind: 'builtin' }, requiredCapabilities: [] } as unknown as ActionTypeDefinition] };

const localTemplate = () => {
  const parsed = parseRuleTemplate({ templateVersion: 1, id: 'local-one', title: 'Local one', actions: [{ name: 'Log', typeId: 'core.log', config: { message: 'hi' } }], event: { name: 'Chat', trigger: 'tiktok.chat' } });
  if (!parsed.ok) throw new Error(parsed.errors.join('; '));
  return { ...parsed.template, custom: true };
};

function mountTemplates(props: { tab?: RuleTemplateTab } = {}) {
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloudLibrary.list') return { items: [], nextCursor: null };
    throw new Error(`unexpected ${method}`);
  });
  const stores = createStoreHarness(client);
  const wrapper = mountWithPlugins(RuleTemplateDialog, {
    props: { open: true, customTemplates: [localTemplate()], snapshot, tab: 'templates' as RuleTemplateTab, 'onUpdate:tab': (value: RuleTemplateTab) => wrapper.setProps({ tab: value }), ...props },
    attachTo: document.body,
    global: { plugins: stores.plugins },
  });
  return wrapper;
}

const templateJson = (id: string, title: string) => ({
  templateVersion: 1, id, title, actions: [{ name: 'Log', typeId: 'core.log', config: { message: 'hi' } }], event: { name: 'Chat', trigger: 'tiktok.chat' },
});

test('the templates modal has Templates, Cloud and Import tabs and starts on the local templates', async () => {
  await activateLocale('en');
  const wrapper = mountTemplates();
  expect(wrapper.findAll('[role="tab"]').map((tab) => tab.text())).toEqual(['Templates', 'Cloud', 'Import']);
  expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Templates');
  expect(wrapper.findAll('.rule-templates__card').length).toBeGreaterThan(0);
  // The old "Import templates…" button is the third tab now, never a second control.
  expect(wrapper.findAll('button').some((button) => button.text() === 'Import templates…')).toBe(false);
  expect(wrapper.find('.cloud-library').exists()).toBe(false);
});

test('the Cloud tab shows the Cloud library without its own title box and imports through it', async () => {
  await activateLocale('en');
  const wrapper = mountTemplates({ tab: 'cloud' });
  await flushPromises();
  expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Cloud');
  expect(wrapper.find('.cloud-library').exists()).toBe(true);
  expect(wrapper.find('.cloud-library__filters').exists()).toBe(true);
  expect(wrapper.find('.rule-templates__card').exists()).toBe(false);
  expect(wrapper.text()).not.toContain('Browse Cloud documents. Viewing a document does not import or activate it.');
});

test('the Import tab stages a template file, reports problems and emits the import with a done callback', async () => {
  await activateLocale('en');
  const wrapper = mountTemplates({ tab: 'manual' });
  expect(wrapper.text()).toContain('Drop a .tiktemplate.json file');
  const drop = wrapper.getComponent(BaseFileDrop);
  drop.vm.$emit('content', '{ not json', 'broken.json');
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toContain('not valid JSON');
  expect(wrapper.get('.base-button--primary').attributes('disabled')).toBeDefined();
  drop.vm.$emit('content', JSON.stringify([templateJson('mine-one', 'Mine one'), templateJson('mine-two', 'Mine two')]), 'pack.json');
  await flushPromises();
  expect(wrapper.get('.base-import-dialog__staged').text()).toContain('Ready to import: 2');
  expect(wrapper.get('.base-import-dialog__staged').text()).toContain('pack.json');
  expect(wrapper.findAll('.base-import-dialog__items li').map((item) => item.text())).toEqual(['Mine one', 'Mine two']);
  expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  await wrapper.get('.base-button--primary').trigger('click');
  const [templates, done] = wrapper.emitted('import')![0] as [Array<{ id: string }>, (added: number) => void];
  expect(templates.map((template) => template.id)).toEqual(['mine-one', 'mine-two']);
  expect(wrapper.get('.base-button--primary').attributes('disabled')).toBeDefined();
  done(2);
  await flushPromises();
  expect(wrapper.get('[role="status"]').text()).toBe('Imported 2.');
  expect(wrapper.find('.base-import-dialog__staged').exists()).toBe(false);
});

test('the Import tab can clear the staged file and refuses an empty one', async () => {
  await activateLocale('en');
  const wrapper = mountTemplates({ tab: 'manual' });
  const drop = wrapper.getComponent(BaseFileDrop);
  drop.vm.$emit('content', '[]', 'empty.json');
  await flushPromises();
  expect(wrapper.get('[role="alert"]').text()).toBe('no templates found');
  drop.vm.$emit('content', JSON.stringify(templateJson('mine-one', 'Mine one')), 'one.json');
  await flushPromises();
  await wrapper.get('button[aria-label="Clear"]').trigger('click');
  expect(wrapper.find('.base-import-dialog__staged').exists()).toBe(false);
});

test('configuring a template replaces the tabs and goes back to the list', async () => {
  await activateLocale('en');
  const wrapper = mountTemplates();
  await wrapper.findAll('.rule-templates__card').find((card) => card.text().includes('Local one'))!.trigger('click');
  expect(wrapper.find('[role="tablist"]').exists()).toBe(false);
  expect(wrapper.find('.rule-templates__card').exists()).toBe(false);
  await wrapper.findAll('.base-dialog__footer button').at(-1)!.trigger('click');
  expect(wrapper.findAll('[role="tab"]').map((tab) => tab.text())).toEqual(['Templates', 'Cloud', 'Import']);
  expect(wrapper.findAll('.rule-templates__card').length).toBeGreaterThan(0);
});
