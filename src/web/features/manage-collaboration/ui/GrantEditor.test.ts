import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';
import { parseGrant } from '@/entities/collaboration/lib/grants.ts';
import GrantEditor from './GrantEditor.vue';

describe('grant restrictions', () => {
  it('keeps reviewer presets read-only and requires explicit workflow write grants', async () => {
    let grant = viewerGrant();
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      await wrapper.findAll('button').find((button) => button.text() === 'Workflow reviewer')!.trigger('click');
      expect(grant.capabilities).toContain('workflows.read');
      expect(grant.capabilities).not.toContain('workflows.toggle');
      expect(grant.capabilities).not.toContain('workflows.edit');
      expect(grant.resources.workflow).toEqual([]);
      await wrapper.findAll('label').find((label) => label.text() === 'Disable workflows (requires read access)')!.find('input').setValue(true);
      expect(grant.capabilities).toContain('workflows.toggle');
      expect(grant.capabilities).not.toContain('workflows.edit');
      expect(grant.resources.workflow).toEqual([]);
    } finally { wrapper.unmount(); }
  });
  it('keeps deny-all distinct from unrestricted and preserves action/setting scopes', async () => {
    let grant: Grant = { ...viewerGrant(), resources: { workflow: ['allowed'] }, actions: { plugin: ['speak'] }, settingsFields: { plugin: ['volume'] } };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const workflows = wrapper.findAll('select')[4]!;
      await workflows.setValue('none'); expect(grant.resources.workflow).toEqual([]);
      await workflows.setValue('all'); expect(grant.resources.workflow).toBeUndefined();
      expect(grant.actions).toEqual({ plugin: ['speak'] }); expect(grant.settingsFields).toEqual({ plugin: ['volume'] });
      await workflows.setValue('selected');
      expect(grant.resources.workflow).toEqual([]);
      const textarea = wrapper.findAll('textarea').find((node) => node.attributes('aria-label')?.startsWith('Workflows'))!;
      await textarea.setValue('first\nsecond\nfirst'); expect(grant.resources.workflow).toEqual(['first', 'second']);
    } finally { wrapper.unmount(); }
  });
  it('sets and removes reusable authorization without widening existing resource rules', async () => {
    let grant: Grant = { ...viewerGrant(), resources: { automation: ['event:allowed'] } };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    const checkbox = (label: string) => wrapper.findAll('label').find((item) => item.text() === label)!.find('input');
    try {
      expect(grant.automationDelegation).toBeUndefined();
      await checkbox('Adjust points').setValue(true);
      expect(grant.automationDelegation).toEqual({ effects: ['points'], allowDynamicTargets: false });
      expect(grant.capabilities).toEqual(expect.arrayContaining(['automation.read', 'automation.edit', 'automation.toggle', 'points.adjust']));
      expect(grant.resources.automation).toEqual(['event:allowed']);
      expect(parseGrant(grant)).toEqual(grant);
      expect(checkbox('Adjust points (requires read access)').attributes('disabled')).toBeDefined();
      await checkbox('Allow supported dynamic targets within the granted limits').setValue(true);
      expect(grant.automationDelegation?.allowDynamicTargets).toBe(true);
      await checkbox('Adjust points').setValue(false);
      expect(grant.automationDelegation).toBeUndefined();
      expect(checkbox('Adjust points (requires read access)').attributes('disabled')).toBeUndefined();
      expect(grant.resources.automation).toEqual(['event:allowed']);
    } finally { wrapper.unmount(); }
  });
  it('edits the points ceiling and rejects invalid limits before saving', async () => {
    let grant: Grant = { ...viewerGrant(), capabilities: ['points.adjust'] };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const input = wrapper.find('input[type="number"]');
      await input.setValue('5.5'); expect(parseGrant(grant).maxPointsDelta).toBe(5.5);
      await input.setValue('0'); expect(() => parseGrant(grant)).toThrow('invalid_grant');
      await input.setValue(''); expect(parseGrant(grant).maxPointsDelta).toBeNull();
    } finally { wrapper.unmount(); }
  });
  it('rejects unknown permissions, wildcards and hidden control characters', () => {
    for (const change of [{ capabilities: ['owner'] }, { resources: { workflow: ['*'] } }, { resources: { workflow: ['hidden\u0080'] } }]) {
      expect(() => parseGrant({ ...viewerGrant(), ...change })).toThrow('invalid_grant');
    }
    expect(parseGrant({ ...viewerGrant(), resources: { workflow: [] } }).resources.workflow).toEqual([]);
  });
});
