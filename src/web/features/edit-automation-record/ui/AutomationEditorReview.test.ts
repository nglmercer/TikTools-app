import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import AutomationEditorReview from './AutomationEditorReview.vue';

enableAutoUnmount(afterEach);

const base = { schemaVersion: 2, id: 'same', name: 'Starting event', enabled: true, trigger: 'tiktok.gift' };
const candidate = { ...base, name: 'My retained draft' };
const current = { ...base, name: 'Host changed name' };
const edit = { revision: '1', base, candidate } as unknown as AutomationEdit;
const review = { current: { record: current, revision: '2' } } as unknown as AutomationReview;

function mountReview(props: Record<string, unknown> = {}) {
  return mountWithPlugins(AutomationEditorReview, {
    attachTo: document.body,
    props: { open: true, edit, review, busy: false, error: '', canAdopt: true, ...props },
  });
}

describe('AutomationEditorReview', () => {
  it('shows the base, candidate and current record each as one pre with the exact pretty JSON', async () => {
    const wrapper = mountReview();
    await nextTick();
    const blocks = wrapper.findAll('pre');
    expect(blocks.map((block) => block.element.textContent)).toEqual([base, candidate, current].map((record) => JSON.stringify(record, null, 2)));
  });

  it('keeps text for an absent base and a deleted host record and offers no diff until asked', async () => {
    const wrapper = mountReview({ edit: { ...edit, base: null }, review: { current: { record: null, revision: '2' } } });
    await nextTick();
    expect(wrapper.findAll('pre')).toHaveLength(1);
    expect(wrapper.text()).toContain('New record')
    expect(wrapper.text()).toContain('The record is absent on the host.');
    expect(wrapper.find('.base-json-diff').exists()).toBe(false);
  });

  it('compares the starting version with the draft and with the host when changes are shown', async () => {
    const wrapper = mountReview();
    await nextTick();
    expect(wrapper.find('.base-json-diff').exists()).toBe(false);
    await wrapper.get('button[aria-expanded="false"]').trigger('click');
    const diffs = wrapper.findAll('.base-json-diff');
    expect(diffs).toHaveLength(2);
    expect(diffs[0]!.text()).toContain('My retained draft');
    expect(diffs[1]!.text()).toContain('Host changed name');
    expect(wrapper.findAll('pre')).toHaveLength(3);
  });
});
