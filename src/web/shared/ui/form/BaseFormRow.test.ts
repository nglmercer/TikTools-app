import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import BaseFormRow from './BaseFormRow.vue';
describe('BaseFormRow associations', () => {
  it('preserves external descriptions while adding and removing hint and error links', async () => {
    const wrapper = mount(BaseFormRow, { props: { label:'Name', for:'name', hint:'A hint', error:'Required' }, slots:{ default:'<input id="name" aria-describedby="external">' } });
    const input = wrapper.get('input');
    expect(input.attributes('aria-describedby')?.split(' ')).toEqual(['external', wrapper.get('.base-form-row__hint').attributes('id'), wrapper.get('[role="alert"]').attributes('id')]);
    expect(input.attributes('aria-invalid')).toBe('true');
    await wrapper.setProps({ hint:undefined, error:undefined });
    expect(input.attributes('aria-describedby')).toBe('external');
    expect(input.attributes('aria-invalid')).toBeUndefined();
  });
  it('uses an existing input id and leaves grouped controls independently labelled', async () => {
    const wrapper = mount(BaseFormRow, { props:{ label:'Name', stacked:true }, slots:{default:'<input id="existing">'} });
    await wrapper.vm.$nextTick();
    expect(wrapper.get('label').attributes('for')).toBe('existing');
    const group = mount(BaseFormRow, { props:{ label:'Choices', group:true, hint:'Choose one' }, slots:{default:'<button aria-label="First">A</button><button aria-label="Second">B</button>'} });
    expect(group.find('label').exists()).toBe(false);
    expect(group.get('[role="group"]').attributes('aria-labelledby')).toBe(group.get('.base-form-row__label').attributes('id'));
    expect(group.get('[role="group"]').attributes('aria-describedby')).toBe(group.get('.base-form-row__hint').attributes('id'));
  });
  it('associates required semantics for existing callers without changing native constraints', async () => {
    const wrapper = mount(BaseFormRow, { props:{label:'Name', required:true}, slots:{default:'<input>'} });
    expect(wrapper.get('input').attributes('aria-required')).toBe('true');
    expect(wrapper.get('input').attributes('required')).toBeUndefined();
    await wrapper.setProps({required:false});
    expect(wrapper.get('input').attributes('aria-required')).toBeUndefined();
  });
  it('does not remove validation owned by the control', async () => {
    const wrapper = mount(BaseFormRow, { props:{label:'Name', error:'Invalid'}, slots:{default:'<input aria-invalid="true">'} });
    await wrapper.setProps({error:undefined});
    expect(wrapper.get('input').attributes('aria-invalid')).toBe('true');
  });
});
