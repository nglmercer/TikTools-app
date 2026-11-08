import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import ProfileImportDialog from './ProfileImportDialog.vue';

enableAutoUnmount(afterEach); afterEach(() => activateLocale('en'));
test('profile aggregate failure stays in the Spanish import dialog with its staged file and editable parameters', async () => {
  await activateLocale('es');
  const template = { templateVersion: 1, id: 'bounded-template', title: 'Bounded', params: { type: 'object', properties: { value: { type: 'string', title: 'Valor', default: 'small' } } },
    actions: Array.from({ length: 4 }, (_, index) => ({ name: `Log ${index}`, typeId: 'core.log', config: { message: '{{ params.value }}' } })), event: { name: 'Chat', trigger: 'tiktok.chat' } };
  const source = JSON.stringify({ profileVersion: 1, id: 'bounded-profile', name: 'Perfil conservado', description: '', params: {}, templates: Array.from({ length: 32 }, () => ({ template, params: {} })) });
  const wrapper = mountWithPlugins(ProfileImportDialog, { props: { open: true }, attachTo: document.body });
  wrapper.getComponent(BaseFileDrop).vm.$emit('content', source, 'perfil.tikprofile.json'); await flushPromises();
  const value = '🌹'.repeat(20_000); const form = wrapper.getComponent(BaseSchemaForm);
  form.vm.$emit('update:modelValue', { value }); await flushPromises();
  await wrapper.get('.base-button--primary').trigger('click');
  expect(wrapper.get('[role="alert"]').text()).toContain('reglas expandidas son demasiado grandes');
  expect(wrapper.text()).toContain('Perfil conservado'); expect(form.props('modelValue')).toEqual({ value }); expect(wrapper.emitted('apply')).toBeUndefined();
  form.vm.$emit('update:modelValue', { value: 'safe' }); await flushPromises();
  await wrapper.get('.base-button--primary').trigger('click'); expect(wrapper.emitted('apply')).toHaveLength(1);
});
