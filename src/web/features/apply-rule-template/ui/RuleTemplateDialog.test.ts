import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import type { ActionTypeDefinition } from '../../../../automation/behavior/types.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { parseRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
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
