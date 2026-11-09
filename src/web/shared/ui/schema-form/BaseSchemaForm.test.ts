import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { defineComponent, h, ref } from 'vue';
import type { JsonObject } from '../../../../automation/types.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseSchemaForm from './BaseSchemaForm.vue';

enableAutoUnmount(afterEach);

const schema: JsonObject = {
  properties: {
    name: { type: 'string', title: 'Name' },
    enabled: { type: 'boolean', title: 'Enabled' },
    mode: { type: 'string', title: 'Mode', enum: ['local', 'remote'] },
    url: { type: 'string', title: 'Server URL' },
    retries: { type: 'integer', title: 'Retries' },
  },
};

const uiHints: JsonObject = {
  fields: {
    url: { showIf: { key: 'mode', equals: ['remote'] } },
    retries: { advanced: true },
  },
};

function mountForm(initial: JsonObject, fieldErrors: Record<string, string> = {}) {
  const values = ref<JsonObject>(initial);
  const Host = defineComponent({
    setup: () => () =>
      h(BaseSchemaForm, {
        schema,
        uiHints,
        fieldErrors,
        modelValue: values.value,
        'onUpdate:modelValue': (next: JsonObject) => (values.value = next),
      }),
  });
  return { wrapper: mountWithPlugins(Host, { attachTo: document.body }), values };
}

describe('BaseSchemaForm', () => {
  it('renders one labelled control per visible field', () => {
    const { wrapper } = mountForm({ mode: 'local' });
    const labels = wrapper.findAll('label').map((label) => label.text());
    expect(labels).toEqual(expect.arrayContaining(['Name', 'Enabled', 'Mode']));
    expect(wrapper.text()).not.toContain('Server URL');
    expect(wrapper.text()).toContain('Advanced options');
  });

  it('writes typed values back to the model', async () => {
    const { wrapper, values } = mountForm({ mode: 'local' });
    await wrapper.get('input[type="text"]').setValue('Studio');
    await wrapper.get('input[type="checkbox"]').setValue(true);
    expect(values.value).toMatchObject({ name: 'Studio', enabled: true, mode: 'local' });
  });

  it('reveals conditional fields when their condition holds', async () => {
    const { wrapper } = mountForm({ mode: 'remote' });
    expect(wrapper.text()).toContain('Server URL');
  });

  it('shows field errors next to the control', () => {
    const { wrapper } = mountForm({ mode: 'remote' }, { url: 'Enter a valid address.' });
    expect(wrapper.text()).toContain('Enter a valid address.');
  });
});


describe('connection settings switches', () => {
  it('opts into switches without changing the default automation controls', () => {
    const schema = { type: 'object', properties: { enabled: { type: 'boolean', title: 'Enabled' } } };
    const switched = mountWithPlugins(BaseSchemaForm, { props: { schema, modelValue: { enabled: true }, switches: true } });
    expect(switched.get('input').attributes('role')).toBe('switch');
    const unchanged = mountWithPlugins(BaseSchemaForm, { props: { schema, modelValue: { enabled: true } } });
    expect(unchanged.get('input').attributes('role')).not.toBe('switch');
  });
});
