import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { i18n } from '@/shared/i18n/i18n.ts';
import { viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';
import { parseGrant } from '@/entities/collaboration/lib/grants.ts';
import GrantEditor from './GrantEditor.vue';

describe('external effect grant editor', () => {
  it('edits exact typed rules without implicitly adding capabilities and removes empty policy', async () => {
    let grant = viewerGrant();
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      expect(Object.keys(grant)).toHaveLength(5);
      await wrapper.findAll('button').find((b) => b.text() === 'Add a bounded HTTP rule')!.trigger('click');
      expect(grant.capabilities).toEqual(['live.read']);
      expect(parseGrant(grant).effects?.http[0]).toMatchObject({ origin: 'https://example.com', paths: ['/'], maxBodyBytes: 0, allowPrivateNetwork: false });
      const origin = wrapper.findAll('label').find((l) => l.text() === 'Exact origin, for example https://hooks.example')!.find('input');
      await origin.setValue('https://hooks.example');
      await wrapper.get('textarea[aria-label="Exact URL paths, one per line"]').setValue('/live\n/live\n/alerts');
      expect(grant.effects?.http[0]?.paths).toEqual(['/live', '/alerts']);
      expect(grant.effects?.http[0]?.origin).toBe('https://hooks.example');
      await wrapper.findAll('button').find((b) => b.text() === 'Remove this rule')!.trigger('click');
      expect(grant.effects).toBeUndefined();
      expect(Object.keys(parseGrant(grant))).toHaveLength(5);
    } finally { wrapper.unmount(); }
  });
  it('requires exact audio references and preserves unrelated grant restrictions', async () => {
    let grant: Grant = { ...viewerGrant(), resources: { viewer: ['alice'] }, actions: { plugin: ['speak'] }, settingsFields: { plugin: ['volume'] } };
    const wrapper = mount(GrantEditor, { props: { modelValue: grant, 'onUpdate:modelValue': async (value: Grant) => { grant = value; await wrapper.setProps({ modelValue: value }); } }, global: { plugins: [i18n] } });
    try {
      await wrapper.findAll('button').find((b) => b.text() === 'Add a bounded audio rule')!.trigger('click');
      expect(() => parseGrant(grant)).toThrow('invalid_grant');
      await wrapper.get('textarea[aria-label="Exact canonical audio file paths on this host, one per line"]').setValue('/media/alert.wav');
      await wrapper.get('textarea[aria-label="Exact lowercase event topics including namespace, one per line"]').setValue('moderator.alert');
      expect(parseGrant(grant).effects?.audio[0]).toEqual({ fileRefs: ['/media/alert.wav'], output: 'host-default', maxVolume: 0.5, overlaps: ['drop'] });
      expect(grant.effects?.emitTopics).toEqual(['moderator.alert']);
      expect(grant.resources.viewer).toEqual(['alice']);
      expect(grant.actions).toEqual({ plugin: ['speak'] });
      expect(grant.settingsFields).toEqual({ plugin: ['volume'] });
      expect(grant.capabilities).toEqual(['live.read']);
    } finally { wrapper.unmount(); }
  });
});
