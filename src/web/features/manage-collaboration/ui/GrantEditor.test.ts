import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { capabilities, viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';
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
  it('adds competition read and command recovery with the stop permission', async () => {
    let grant: Grant = { ...viewerGrant(), capabilities: [] };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      await wrapper.findAll('label').find((item) => item.text() === 'Manage competitions (includes read and command recovery)')!.find('input').setValue(true);
      expect(grant.capabilities).toEqual(expect.arrayContaining(['competitions.read','competitions.manage','commands.read']));
      expect(parseGrant(grant)).toEqual(grant);
    } finally { wrapper.unmount(); }
  });
  it('adds the read and recovery capabilities needed by permitted changes', async () => {
    let grant: Grant = { ...viewerGrant(), capabilities: [] };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    const checkbox = (label: string) => wrapper.findAll('label').find((item) => item.text() === label)!.find('input');
    try {
      await checkbox('Enable or disable permitted automations (includes read and recovery)').setValue(true);
      expect(grant.capabilities).toEqual(expect.arrayContaining(['automation.toggle', 'automation.read', 'commands.read']));
      expect(checkbox('Read automation status').attributes('disabled')).toBeDefined();
      await checkbox('Adjust points (requires read access)').setValue(true);
      expect(grant.capabilities).toEqual(expect.arrayContaining(['points.adjust', 'points.read', 'commands.read']));
      expect(parseGrant(grant)).toEqual(grant);
    } finally { wrapper.unmount(); }
  });
  it('keeps deny-all distinct from unrestricted and preserves action/setting scopes', async () => {
    let grant: Grant = { ...viewerGrant(), resources: { workflow: ['allowed'] }, actions: { plugin: ['speak'] }, settingsFields: { plugin: ['volume'] } };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const workflows = wrapper.get('[role="radiogroup"][aria-label="Workflows"]');
      const mode = (text: string) => workflows.findAll('button').find(button => button.text() === text)!;
      await mode('None').trigger('click'); expect(grant.resources.workflow).toEqual([]);
      await mode('All').trigger('click'); expect(grant.resources.workflow).toBeUndefined();
      expect(grant.actions).toEqual({ plugin: ['speak'] }); expect(grant.settingsFields).toEqual({ plugin: ['volume'] });
      await mode('Selected IDs').trigger('click');
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
      expect(grant.capabilities).toEqual(expect.arrayContaining(['automation.read', 'automation.edit', 'automation.toggle', 'points.read', 'points.adjust']));
      expect(grant.resources.automation).toEqual(['event:allowed']);
      expect(parseGrant(grant)).toEqual(grant);
      expect(checkbox('Adjust points (requires read access)').attributes('disabled')).toBeDefined();
      expect(checkbox('Read points').attributes('disabled')).toBeDefined();
      await checkbox('Allow supported dynamic targets within the granted limits').setValue(true);
      expect(grant.automationDelegation?.allowDynamicTargets).toBe(true);
      await checkbox('Adjust points').setValue(false);
      expect(grant.automationDelegation).toBeUndefined();
      expect(checkbox('Adjust points (requires read access)').attributes('disabled')).toBeUndefined();
      expect(grant.resources.automation).toEqual(['event:allowed']);
    } finally { wrapper.unmount(); }
  });
  it('adds and retains LIVE read while reusable script execution is selected', async () => {
    let grant: Grant = { ...viewerGrant(), capabilities: [] };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    const checkbox = (label: string) => wrapper.findAll('label').find((item) => item.text() === label)!.find('input');
    try {
      await checkbox('Run scripts').setValue(true);
      expect(grant.capabilities).toEqual(expect.arrayContaining(['live.read', 'scripts.execute', 'automation.read', 'automation.edit', 'automation.toggle']));
      expect(grant.automationDelegation?.allowDynamicTargets).toBe(false);
      expect(checkbox('Read LIVE status').attributes('disabled')).toBeDefined();
      await checkbox('Run scripts').setValue(false);
      expect(checkbox('Read LIVE status').attributes('disabled')).toBeUndefined();
    } finally { wrapper.unmount(); }
  });
  it('allows adding a missing execution capability in an existing grant', async () => {
    let grant: Grant = { ...viewerGrant(), capabilities:['automation.read','automation.edit','automation.toggle','scripts.execute'], automationDelegation:{effects:['script'],allowDynamicTargets:true} };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const read = wrapper.findAll('label').find((item) => item.text() === 'Read LIVE status')!.find('input');
      expect(read.attributes('disabled')).toBeUndefined();
      await read.setValue(true);
      expect(grant.capabilities).toContain('live.read');
      expect(read.attributes('disabled')).toBeDefined();
    } finally { wrapper.unmount(); }
  });
  it('requires an explicit choice to allow changes to owner definitions', async () => {
    let grant = viewerGrant();
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    const checkbox = (label: string) => wrapper.findAll('label').find((item) => item.text() === label)!.find('input');
    try {
      const share = checkbox('Allow edits to existing owner automations within this scope');
      expect(share.attributes('disabled')).toBeDefined();
      await checkbox('Log messages').setValue(true);
      expect(grant.automationDelegation?.allowOwnerEdits).toBeUndefined();
      await share.setValue(true);
      expect(parseGrant(grant).automationDelegation?.allowOwnerEdits).toBe(true);
      const source = checkbox('Allow this identity to read owner automation source');
      await source.setValue(true);
      expect(parseGrant(grant).automationDelegation?.allowOwnerSource).toBe(true);
      await checkbox('Delay').setValue(true);
      expect(grant.automationDelegation?.allowOwnerEdits).toBe(true);
      await share.setValue(false);
      expect(grant.automationDelegation?.allowOwnerEdits).toBeUndefined();
      expect(grant.automationDelegation?.allowOwnerSource).toBeUndefined();
      const other = checkbox('Allow edits to other moderators’ automations within this scope');
      const otherSource = checkbox('Allow reading other moderators’ definitions');
      expect(otherSource.attributes('disabled')).toBeDefined();
      await other.setValue(true);
      await otherSource.setValue(true);
      expect(parseGrant(grant).automationDelegation?.allowOtherAuthorSource).toBe(true);
      await other.setValue(false);
      expect(grant.automationDelegation?.allowOtherAuthorEdits).toBeUndefined();
      expect(grant.automationDelegation?.allowOtherAuthorSource).toBeUndefined();
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

describe('bulk permission edits', () => {
  it('enables capabilities without widening resource or effect restrictions and clears delegation on disable', async () => {
    let grant: Grant = { ...viewerGrant(), resources: { workflow: ['one'] }, actions: { plugin: ['speak'] }, settingsFields: { plugin: ['volume'] }, maxPointsDelta: 10, automationDelegation: { effects: ['log'], allowDynamicTargets: false }, capabilities: ['automation.read', 'automation.edit', 'automation.toggle'] };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      await wrapper.findAll('button').find(button => button.text() === 'Enable all permissions')!.trigger('click');
      expect(new Set(grant.capabilities)).toEqual(new Set(capabilities));
      expect(grant.resources).toEqual({ workflow: ['one'] });
      expect(grant.actions).toEqual({ plugin: ['speak'] });
      expect(grant.settingsFields).toEqual({ plugin: ['volume'] });
      expect(grant.maxPointsDelta).toBe(10);
      expect(grant.effects).toBeUndefined();
      expect(parseGrant(grant)).toEqual(grant);
      await wrapper.findAll('button').find(button => button.text() === 'Disable all permissions')!.trigger('click');
      expect(grant.capabilities).toEqual([]);
      expect(grant.automationDelegation).toBeUndefined();
      expect(grant.resources).toEqual({ workflow: ['one'] });
      expect(parseGrant(grant)).toEqual(grant);
    } finally { wrapper.unmount(); }
  });
  it('does not allow bulk edits when disabled', async () => {
    const wrapper = mount(GrantEditor, { props: { modelValue: viewerGrant(), disabled: true }, global: { plugins: [i18n] } });
    try {
      for (const name of ['Enable all permissions', 'Disable all permissions']) {
        const button = wrapper.findAll('button').find(button => button.text() === name)!;
        expect(button.attributes('disabled')).toBeDefined();
        await button.trigger('click');
      }
      expect(wrapper.emitted('update:modelValue')).toBeUndefined();
    } finally { wrapper.unmount(); }
  });
});

describe('grant panel draft boundaries', () => {
  it('resets an empty selected-ID scope when a reviewed JSON draft denies that resource', async () => {
    let grant = viewerGrant();
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const scope = wrapper.get('[role="radiogroup"][aria-label="Workflows"]');
      await scope.findAll('button').find(button => button.text() === 'Selected IDs')!.trigger('click');
      const raw = wrapper.get('textarea[aria-label="Advanced grant JSON"]');
      await raw.setValue(JSON.stringify({ ...grant, resources: { workflow: [] } }));
      await wrapper.findAll('button').find(button => button.text() === 'Use this JSON draft')!.trigger('click');
      expect(scope.findAll('button').find(button => button.text() === 'None')!.attributes('aria-checked')).toBe('true');
      expect(grant.resources.workflow).toEqual([]);
    } finally { wrapper.unmount(); }
  });
  it('keeps dirty JSON while editing a capability and resets it when a preset is chosen', async () => {
    let grant = viewerGrant();
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      const raw = wrapper.get('textarea[aria-label="Advanced grant JSON"]');
      await raw.setValue('{unfinished');
      await wrapper.findAll('label').find(label => label.text() === 'Read points')!.find('input').setValue(true);
      expect((raw.element as HTMLTextAreaElement).value).toBe('{unfinished');
      await wrapper.findAll('button').find(button => button.text() === 'Workflow reviewer')!.trigger('click');
      expect(JSON.parse((raw.element as HTMLTextAreaElement).value)).toEqual(grant);
      expect(grant.resources.workflow).toEqual([]);
    } finally { wrapper.unmount(); }
  });
});
